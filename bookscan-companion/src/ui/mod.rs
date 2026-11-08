use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use egui::{Color32, CornerRadius, Frame, RichText, Sense, Stroke, TextureHandle, Vec2};
use tokio::sync::broadcast;

use crate::app_state::{AppEvent, SharedState};
use crate::capture_control::{CaptureMode, PointState};
use crate::config::{self, AppConfig};
use crate::pdf_export;
use crate::pipeline::{EnhanceMode, PipelineConfig};

const ACCENT: Color32 = Color32::from_rgb(50, 205, 177);

pub struct CompanionApp {
    shared: SharedState,
    events: broadcast::Receiver<AppEvent>,
    status_line: String,
    listen_addr: String,
    local_ip: String,
    library_root: PathBuf,
    processed: Vec<PathBuf>,
    /// Index of the left panel page (0-based). Right shows view_index+1 when possible.
    view_index: usize,
    /// Which panel is selected: 0 = left, 1 = right.
    selected_side: usize,
    thumb_tex: HashMap<String, TextureHandle>,
    left_tex: Option<TextureHandle>,
    right_tex: Option<TextureHandle>,
    left_path: Option<PathBuf>,
    right_path: Option<PathBuf>,
    pipeline: PipelineConfig,
    dark: bool,
    last_refresh: std::time::Instant,
    needs_texture_refresh: bool,
    confirm_delete: bool,
}

impl CompanionApp {
    pub fn new(cc: &eframe::CreationContext<'_>, shared: SharedState) -> Self {
        let events = shared.event_tx.subscribe();
        let listen_addr = shared.status_snapshot().listen_addr;
        let local_ip = crate::net_if::preferred_lan_ipv4();
        let pipeline = shared.pipeline();
        let library_root = shared.library_root();
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        cc.egui_ctx.set_style(style);

        let mut app = Self {
            shared,
            events,
            status_line: format!("dossier: {}", library_root.display()),
            listen_addr,
            local_ip,
            library_root,
            processed: Vec::new(),
            view_index: 0,
            selected_side: 1,
            thumb_tex: HashMap::new(),
            left_tex: None,
            right_tex: None,
            left_path: None,
            right_path: None,
            pipeline,
            dark: true,
            last_refresh: std::time::Instant::now() - Duration::from_secs(10),
            needs_texture_refresh: true,
            confirm_delete: true,
        };
        app.refresh_processed(&cc.egui_ctx);
        app
    }

    fn persist_config(&self) {
        let cfg = AppConfig {
            library_root: self.library_root.clone(),
            pipeline: self.pipeline.clone(),
        };
        let _ = config::save_config(&cfg);
    }

    fn pick_library_folder(&mut self) {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title("Dossier d'enregistrement bookscan")
            .set_directory(&self.library_root)
            .pick_folder()
        {
            match self.shared.set_library_root(dir.clone()) {
                Ok(()) => {
                    self.library_root = dir;
                    self.persist_config();
                    self.view_index = 0;
                    self.thumb_tex.clear();
                    self.needs_texture_refresh = true;
                    self.status_line = format!("dossier: {}", self.library_root.display());
                }
                Err(e) => self.status_line = format!("dossier invalide: {e:#}"),
            }
        }
    }

    fn poll_events(&mut self) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                AppEvent::Status(s) => self.status_line = s,
                AppEvent::PageUpdated => {
                    self.needs_texture_refresh = true;
                }
                AppEvent::PhoneConnected(c) => {
                    self.status_line = if c {
                        "téléphone connecté".into()
                    } else {
                        "téléphone déconnecté".into()
                    };
                }
                AppEvent::PointState(s) => {
                    self.status_line = format!("point: {}", s.label());
                }
            }
        }
    }

    fn go_live(&mut self) {
        let n = self.processed.len();
        self.view_index = n.saturating_sub(2);
        if n == 1 {
            self.view_index = 0;
        }
        self.selected_side = if n >= 2 { 1 } else { 0 };
        self.needs_texture_refresh = true;
    }

    fn selected_page_index(&self) -> Option<usize> {
        let n = self.processed.len();
        if n == 0 {
            return None;
        }
        let idx = self.view_index + self.selected_side;
        if idx < n {
            Some(idx)
        } else if self.view_index < n {
            Some(self.view_index)
        } else {
            None
        }
    }

    fn delete_selected(&mut self) {
        let Some(idx) = self.selected_page_index() else {
            return;
        };
        let path = self.processed[idx].clone();
        if self.confirm_delete {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let ok = rfd::MessageDialog::new()
                .set_title("Supprimer la page")
                .set_description(format!("Envoyer « {name} » dans trash ?"))
                .set_buttons(rfd::MessageButtons::OkCancel)
                .show()
                == rfd::MessageDialogResult::Ok;
            if !ok {
                return;
            }
        }
        match self.shared.delete_processed_page(&path) {
            Ok(()) => {
                self.thumb_tex.remove(&path.display().to_string());
                self.needs_texture_refresh = true;
            }
            Err(e) => self.status_line = format!("suppression: {e:#}"),
        }
    }

    fn move_selected(&mut self, delta: i32) {
        let Some(idx) = self.selected_page_index() else {
            return;
        };
        match self.shared.move_processed_page(idx, delta) {
            Ok(_) => {
                self.thumb_tex.clear();
                let new_idx = (idx as i32 + delta).max(0) as usize;
                // Keep selection on the moved page
                if new_idx == 0 {
                    self.view_index = 0;
                    self.selected_side = 0;
                } else {
                    self.view_index = new_idx - 1;
                    self.selected_side = 1;
                }
                self.needs_texture_refresh = true;
            }
            Err(e) => self.status_line = format!("déplacer: {e:#}"),
        }
    }

    fn refresh_processed(&mut self, ctx: &egui::Context) {
        let prev_len = self.processed.len();
        self.processed = self.shared.list_processed();
        let n = self.processed.len();

        // Auto-follow live end when new pages arrive
        if n > prev_len {
            self.go_live();
        }
        if self.view_index >= n && n > 0 {
            self.view_index = n.saturating_sub(2);
        }

        let left = if n == 0 {
            None
        } else if n == 1 {
            Some(self.processed[0].clone())
        } else {
            let i = self.view_index.min(n - 2);
            self.view_index = i;
            Some(self.processed[i].clone())
        };
        let right = if n >= 2 {
            let i = self.view_index.min(n - 2);
            Some(self.processed[i + 1].clone())
        } else {
            None
        };

        if left != self.left_path {
            self.left_path = left.clone();
            self.left_tex = left.as_ref().and_then(|p| load_texture(ctx, "left", p, 1400));
        }
        if right != self.right_path {
            self.right_path = right.clone();
            self.right_tex = right.as_ref().and_then(|p| load_texture(ctx, "right", p, 1400));
        }

        // Refresh visible thumbs
        for (i, p) in self.processed.iter().enumerate().rev().take(30) {
            let key = p.display().to_string();
            if !self.thumb_tex.contains_key(&key) {
                if let Some(tex) = load_texture(ctx, &format!("th{i}"), p, 256) {
                    self.thumb_tex.insert(key, tex);
                }
            }
        }
        self.last_refresh = std::time::Instant::now();
    }

    fn apply_pipeline_settings(&mut self) {
        self.shared.set_pipeline(self.pipeline.clone());
        self.persist_config();
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.shared.request_capture();
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp) {
                if i.modifiers.ctrl {
                    self.move_selected(-1);
                } else if self.selected_side == 1 && self.processed.len() >= 2 {
                    self.selected_side = 0;
                } else if self.view_index > 0 {
                    self.view_index -= 1;
                    self.needs_texture_refresh = true;
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown) {
                if i.modifiers.ctrl {
                    self.move_selected(1);
                } else if self.selected_side == 0 && self.processed.len() >= 2 {
                    self.selected_side = 1;
                } else if self.view_index + 2 < self.processed.len() {
                    self.view_index += 1;
                    self.needs_texture_refresh = true;
                }
            }
            if i.key_pressed(egui::Key::Home) {
                self.view_index = 0;
                self.selected_side = 0;
                self.needs_texture_refresh = true;
            }
            if i.key_pressed(egui::Key::End) {
                self.go_live();
            }
            if i.key_pressed(egui::Key::Tab) {
                self.selected_side = 1 - self.selected_side;
            }
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                self.delete_selected();
            }
        });
    }
}

impl eframe::App for CompanionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        if self.needs_texture_refresh || self.last_refresh.elapsed() > Duration::from_millis(900) {
            self.left_path = None;
            self.right_path = None;
            self.refresh_processed(ctx);
            self.needs_texture_refresh = false;
        }
        self.handle_keys(ctx);
        ctx.request_repaint_after(Duration::from_millis(200));

        let snap = self.shared.status_snapshot();
        let n = self.processed.len();
        let left_num = if n == 0 {
            None
        } else if n == 1 {
            Some(1usize)
        } else {
            Some(self.view_index + 1)
        };
        let right_num = if n >= 2 {
            Some(self.view_index + 2)
        } else {
            None
        };

        egui::TopBottomPanel::top("top")
            .frame(Frame::new().fill(Color32::from_rgb(18, 20, 23)).inner_margin(10.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("bookscan companion").color(ACCENT).strong());
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("{}:{}", self.local_ip, port_of(&self.listen_addr))).monospace());
                    ui.separator();
                    if snap.phone_connected {
                        ui.label(RichText::new("● téléphone").color(ACCENT));
                    } else {
                        ui.label(RichText::new("○ téléphone").color(Color32::from_rgb(180, 80, 80)));
                    }
                    ui.label(point_badge(snap.point_state));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Live (End)").clicked() {
                            self.go_live();
                        }
                        if ui.button("Capturer (Space)").clicked() {
                            self.shared.request_capture();
                        }
                    });
                });
            });

        egui::TopBottomPanel::bottom("bottom")
            .exact_height(148.0)
            .frame(Frame::new().fill(Color32::from_rgb(22, 26, 32)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Historique").strong());
                    ui.label(
                        RichText::new("←/→ nav · Tab panneau · Del supprimer · Ctrl+←/→ déplacer")
                            .small()
                            .color(Color32::GRAY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{n} pages · queue {}", snap.queue_len));
                    });
                });
                ui.add_space(4.0);
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, p) in self.processed.iter().enumerate().rev() {
                                let selected = self.selected_page_index() == Some(i);
                                let key = p.display().to_string();
                                let mut frame = Frame::new()
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(4.0)
                                    .fill(Color32::from_rgb(30, 34, 40));
                                if selected {
                                    frame = frame.stroke(Stroke::new(2.0_f32, ACCENT));
                                }
                                let resp = frame.show(ui, |ui| {
                                    ui.set_width(84.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}", i + 1))
                                                .small()
                                                .color(if selected { ACCENT } else { Color32::GRAY }),
                                        );
                                        if let Some(tex) = self.thumb_tex.get(&key) {
                                            ui.add(
                                                egui::Image::new(tex)
                                                    .fit_to_exact_size(Vec2::new(72.0, 96.0)),
                                            );
                                        } else {
                                            ui.allocate_exact_size(
                                                Vec2::new(72.0, 96.0),
                                                Sense::hover(),
                                            );
                                        }
                                    });
                                });
                                let click = ui.allocate_rect(resp.response.rect, Sense::click());
                                if click.clicked() {
                                    if i == 0 {
                                        self.view_index = 0;
                                        self.selected_side = 0;
                                    } else {
                                        self.view_index = i - 1;
                                        self.selected_side = 1;
                                    }
                                    self.needs_texture_refresh = true;
                                }
                                ui.add_space(6.0);
                            }
                        });
                    });
                ui.horizontal(|ui| {
                    ui.label(&self.status_line);
                });
            });

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(250.0)
            .frame(Frame::new().fill(Color32::from_rgb(20, 24, 28)).inner_margin(10.0))
            .show(ctx, |ui| {
                ui.heading("Capture");
                ui.horizontal(|ui| {
                    for mode in [CaptureMode::Auto, CaptureMode::Semi, CaptureMode::Manual] {
                        let selected = snap.capture_mode == mode;
                        if ui.selectable_label(selected, mode.as_str()).clicked() {
                            self.shared.set_capture_mode(mode);
                        }
                    }
                });

                ui.add_space(10.0);
                ui.heading("Page sélectionnée");
                ui.horizontal(|ui| {
                    if ui.button("⟵ Plus tôt").clicked() {
                        self.move_selected(-1);
                    }
                    if ui.button("Plus tard ⟶").clicked() {
                        self.move_selected(1);
                    }
                });
                if ui
                    .add(egui::Button::new(RichText::new("Supprimer").color(Color32::from_rgb(255, 120, 120))))
                    .clicked()
                {
                    self.delete_selected();
                }
                ui.checkbox(&mut self.confirm_delete, "Confirmer suppression");

                ui.add_space(10.0);
                ui.heading("Enregistrement");
                ui.label(
                    RichText::new(self.library_root.display().to_string())
                        .small()
                        .monospace(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Choisir…").clicked() {
                        self.pick_library_folder();
                    }
                    if ui.button("Ouvrir").clicked() {
                        let _ = opener::open(&self.library_root);
                    }
                });

                ui.add_space(10.0);
                ui.heading("Pipeline");
                ui.checkbox(&mut self.pipeline.detect, "detect");
                ui.checkbox(&mut self.pipeline.dewarp, "dewarp");
                ui.checkbox(&mut self.pipeline.finger, "finger");
                ui.checkbox(&mut self.pipeline.handwriting, "handwriting");
                egui::ComboBox::from_label("enhance")
                    .selected_text(self.pipeline.enhance.label())
                    .show_ui(ui, |ui| {
                        for m in EnhanceMode::all() {
                            ui.selectable_value(&mut self.pipeline.enhance, m, m.label());
                        }
                    });
                if ui.button("Appliquer").clicked() {
                    self.apply_pipeline_settings();
                }

                ui.add_space(10.0);
                if ui.button("Export PDF").clicked() {
                    let pages = self.shared.list_processed();
                    let out = self.library_root.join("bookscan-export.pdf");
                    match pdf_export::export_pages_to_pdf(&pages, &out) {
                        Ok(()) => self.status_line = format!("PDF → {}", out.display()),
                        Err(e) => self.status_line = format!("PDF error: {e:#}"),
                    }
                }
                if ui
                    .button(if self.dark {
                        "Thème clair"
                    } else {
                        "Thème sombre"
                    })
                    .clicked()
                {
                    self.dark = !self.dark;
                    let mut style = (*ctx.style()).clone();
                    style.visuals = if self.dark {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    };
                    ctx.set_style(style);
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::from_rgb(14, 16, 19)).inner_margin(10.0))
            .show(ctx, |ui| {
                let avail = ui.available_size();
                let gap = 12.0;
                let panel_w = ((avail.x - gap) * 0.5).max(200.0);
                let panel_h = (avail.y - 8.0).max(240.0);

                let mut clicked_side: Option<usize> = None;
                ui.horizontal(|ui| {
                    if preview_panel(
                        ui,
                        left_num,
                        self.left_tex.as_ref(),
                        self.left_path.as_ref(),
                        self.selected_side == 0,
                        Vec2::new(panel_w, panel_h),
                    ) {
                        clicked_side = Some(0);
                    }
                    ui.add_space(gap);
                    if preview_panel(
                        ui,
                        right_num,
                        self.right_tex.as_ref(),
                        self.right_path.as_ref(),
                        self.selected_side == 1,
                        Vec2::new(panel_w, panel_h),
                    ) {
                        clicked_side = Some(1);
                    }
                });
                if let Some(side) = clicked_side {
                    self.selected_side = side;
                }
            });
    }
}

fn preview_panel(
    ui: &mut egui::Ui,
    page_num: Option<usize>,
    tex: Option<&TextureHandle>,
    path: Option<&PathBuf>,
    selected: bool,
    size: Vec2,
) -> bool {
    let stroke = if selected {
        Stroke::new(2.0_f32, ACCENT)
    } else {
        Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 58))
    };
    let frame = Frame::new()
        .fill(Color32::from_rgb(26, 30, 36))
        .stroke(stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(10.0);

    let resp = frame.show(ui, |ui| {
        ui.set_min_size(size);
        ui.horizontal(|ui| {
            if let Some(n) = page_num {
                ui.label(RichText::new(format!("Page {n}")).strong().size(18.0));
            } else {
                ui.label(RichText::new("—").strong().size(18.0));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(name) = path.and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned())) {
                    ui.label(RichText::new(name).small().color(Color32::GRAY));
                }
            });
        });
        ui.add_space(6.0);
        let img_size = Vec2::new(size.x - 20.0, size.y - 48.0);
        if let Some(t) = tex {
            ui.add(egui::Image::new(t).fit_to_exact_size(img_size));
        } else {
            ui.allocate_ui(img_size, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Aucune image").color(Color32::GRAY));
                });
            });
        }
    });
    ui.allocate_rect(resp.response.rect, Sense::click()).clicked()
}

fn load_texture(ctx: &egui::Context, name: &str, path: &PathBuf, max_side: u32) -> Option<TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    let resized = if w.max(h) > max_side {
        let scale = max_side as f32 / w.max(h) as f32;
        image::imageops::resize(
            &img,
            ((w as f32) * scale) as u32,
            ((h as f32) * scale) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let size = [resized.width() as usize, resized.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &resized.into_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

fn port_of(addr: &str) -> String {
    addr.rsplit(':').next().unwrap_or("7842").to_string()
}

fn point_badge(state: PointState) -> RichText {
    let (label, color) = match state {
        PointState::Armed => ("● armed", ACCENT),
        PointState::Stable => ("● stable", Color32::YELLOW),
        PointState::Searching => ("● searching", Color32::GRAY),
        PointState::Unknown => ("● —", Color32::DARK_GRAY),
    };
    RichText::new(label).color(color)
}
