package com.atelier.bookscan

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.TimeUnit

class CompanionClient(
    private val scope: CoroutineScope,
    private val onStatus: (String) -> Unit,
    private val onMode: (CaptureMode) -> Unit,
    private val onAutoParams: (AutoParams) -> Unit,
    private val onCaptureNow: () -> Unit,
) {
    private val client = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .pingInterval(20, TimeUnit.SECONDS)
        .build()

    @Volatile
    private var baseUrl: String = ""

    @Volatile
    private var ws: WebSocket? = null

    fun connect(hostPort: String) {
        val cleaned = hostPort.trim()
            .removePrefix("http://")
            .removePrefix("https://")
            .trimEnd('/')
        baseUrl = "http://$cleaned"
        val wsUrl = "ws://$cleaned/v1/ws"
        ws?.cancel()
        val req = Request.Builder().url(wsUrl).build()
        ws = client.newWebSocket(req, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                onStatus("WS connected")
                webSocket.send(JSONObject().put("type", "hello").put("device", "bookscan").put("version", "0.1.0").toString())
                webSocket.send(JSONObject().put("type", "ready").toString())
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleWs(text)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                onStatus("WS error: ${t.message}")
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                onStatus("WS closed")
            }
        })
        scope.launch(Dispatchers.IO) {
            try {
                val statusReq = Request.Builder().url("$baseUrl/v1/status").get().build()
                client.newCall(statusReq).execute().use { resp ->
                    if (resp.isSuccessful) {
                        onStatus("HTTP ok · ${resp.body?.string()?.take(120)}")
                    } else {
                        onStatus("HTTP ${resp.code}")
                    }
                }
            } catch (e: Exception) {
                onStatus("HTTP fail: ${e.message}")
            }
        }
    }

    fun disconnect() {
        ws?.close(1000, "bye")
        ws = null
    }

    fun sendPointState(state: PointState) {
        ws?.send(JSONObject().put("type", "point_state").put("state", state.toWire()).toString())
    }

    fun sendCapturing() {
        ws?.send(JSONObject().put("type", "capturing").toString())
    }

    fun uploadJpeg(bytes: ByteArray, onDone: (Boolean, String) -> Unit) {
        val url = baseUrl
        if (url.isBlank()) {
            onDone(false, "not connected")
            return
        }
        scope.launch(Dispatchers.IO) {
            try {
                val pageId = UUID.randomUUID().toString()
                val body = MultipartBody.Builder()
                    .setType(MultipartBody.FORM)
                    .addFormDataPart("page_id", pageId)
                    .addFormDataPart(
                        "file",
                        "$pageId.jpg",
                        bytes.toRequestBody("image/jpeg".toMediaType()),
                    )
                    .build()
                val req = Request.Builder().url("$url/v1/pages").post(body).build()
                client.newCall(req).execute().use { resp ->
                    val ok = resp.isSuccessful
                    val msg = resp.body?.string().orEmpty()
                    if (ok) {
                        ws?.send(JSONObject().put("type", "uploaded").put("page_id", pageId).toString())
                    }
                    onDone(ok, if (ok) pageId else "upload ${resp.code}: $msg")
                }
            } catch (e: Exception) {
                onDone(false, e.message ?: "upload failed")
            }
        }
    }

    private fun handleWs(text: String) {
        try {
            val obj = JSONObject(text)
            when (obj.optString("type")) {
                "capture_now" -> onCaptureNow()
                "set_mode" -> onMode(CaptureMode.fromWire(obj.optString("mode", "semi")))
                "set_auto_params" -> onAutoParams(
                    AutoParams(
                        settleMs = obj.optLong("settle_ms", 600),
                        cooldownMs = obj.optLong("cooldown_ms", 1200),
                        motionThreshold = obj.optDouble("motion_threshold", 12.0).toFloat(),
                    ),
                )
            }
        } catch (e: Exception) {
            onStatus("bad ws: ${e.message}")
        }
    }
}
