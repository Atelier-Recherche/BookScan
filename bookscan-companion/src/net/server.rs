use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::app_state::SharedState;
use crate::capture_control::{PointState, WsInbound, WsOutbound};

#[derive(Clone)]
struct AppStateHttp {
    shared: SharedState,
}

pub async fn start_server(shared: SharedState, addr: SocketAddr) -> anyhow::Result<()> {
    let state = AppStateHttp {
        shared: shared.clone(),
    };
    let app = Router::new()
        .route("/v1/status", get(status))
        .route("/v1/pages", post(upload_page))
        .route("/v1/pages/{id}", get(page_status))
        .route("/v1/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .layer(DefaultBodyLimit::max(40 * 1024 * 1024))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    info!("bookscan companion listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
struct StatusJson {
    ok: bool,
    version: String,
    queue_len: usize,
    processed_count: usize,
    phone_connected: bool,
    capture_mode: String,
    point_state: String,
}

async fn status(State(state): State<AppStateHttp>) -> Json<StatusJson> {
    let s = state.shared.status_snapshot();
    Json(StatusJson {
        ok: s.ok,
        version: s.version,
        queue_len: s.queue_len,
        processed_count: s.processed_count,
        phone_connected: s.phone_connected,
        capture_mode: s.capture_mode.as_str().to_string(),
        point_state: s.point_state.label().to_string(),
    })
}

#[derive(Serialize)]
struct UploadResp {
    page_id: String,
    queued: bool,
}

async fn upload_page(
    State(state): State<AppStateHttp>,
    mut multipart: Multipart,
) -> Result<Json<UploadResp>, (axum::http::StatusCode, String)> {
    let mut page_id: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut ext = "jpg".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "page_id" {
            page_id = Some(
                field
                    .text()
                    .await
                    .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?,
            );
        } else if name == "file" {
            if let Some(filename) = field.file_name().map(|s| s.to_string()) {
                if let Some(e) = std::path::Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                {
                    ext = e.to_ascii_lowercase();
                }
            }
            file_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
                    .to_vec(),
            );
        }
    }

    let bytes = file_bytes.ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "missing file field".into(),
    ))?;

    let rec = state
        .shared
        .enqueue_bytes(page_id, &bytes, &ext)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UploadResp {
        page_id: rec.id,
        queued: true,
    }))
}

#[derive(Serialize)]
struct PageStatusJson {
    page_id: String,
    status: String,
    error: Option<String>,
}

async fn page_status(
    State(state): State<AppStateHttp>,
    Path(id): Path<String>,
) -> Result<Json<PageStatusJson>, axum::http::StatusCode> {
    let rec = state
        .shared
        .get_page(&id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    let status = match rec.status {
        crate::storage::PageStatus::Queued => "queued",
        crate::storage::PageStatus::Processing => "processing",
        crate::storage::PageStatus::Ready => "ready",
        crate::storage::PageStatus::Error => "error",
    };
    Ok(Json(PageStatusJson {
        page_id: rec.id,
        status: status.into(),
        error: rec.error,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppStateHttp>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state.shared))
}

async fn handle_ws(socket: WebSocket, shared: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    shared.set_phone_connected(true);
    info!("phone websocket connected");

    let mut capture_rx = shared.capture_tx.subscribe();
    let shared_out = shared.clone();

    let send_task = tokio::spawn(async move {
        // Push current mode/params on connect
        let mode = shared_out.capture_mode();
        let params = shared_out.auto_params();
        let msgs = [
            WsOutbound::SetMode { mode },
            WsOutbound::SetAutoParams {
                settle_ms: params.settle_ms,
                cooldown_ms: params.cooldown_ms,
                motion_threshold: params.motion_threshold,
            },
        ];
        for msg in msgs {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text.into())).await.is_err() {
                    return;
                }
            }
        }

        while let Ok(msg) = capture_rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<WsInbound>(&text) {
                    Ok(WsInbound::Hello { .. }) | Ok(WsInbound::Ready) => {
                        shared.set_phone_connected(true);
                    }
                    Ok(WsInbound::PointState { state }) => {
                        shared.set_point_state(PointState::from_str_lossy(&state));
                    }
                    Ok(WsInbound::Capturing) => {
                        let _ = shared
                            .event_tx
                            .send(crate::app_state::AppEvent::Status("capturing…".into()));
                    }
                    Ok(WsInbound::Uploaded { page_id }) => {
                        let _ = shared.event_tx.send(crate::app_state::AppEvent::Status(
                            format!("uploaded {page_id}"),
                        ));
                    }
                    Ok(WsInbound::Error { message }) => {
                        warn!("phone error: {message}");
                    }
                    Err(e) => warn!("bad ws json: {e} :: {text}"),
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    shared.set_phone_connected(false);
    shared.set_point_state(PointState::Unknown);
    info!("phone websocket disconnected");
}
