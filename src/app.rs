use area_selector::config::{self, AppConfig, AutoSave, AutoSaveAction, LUPA_FPS_RANGE};
use area_selector::lupa::{LupaApp, LupaOutcome};
use area_selector::model::Area;
use area_selector::ui_selector::{self, AreaSelector, SelectorOutcome};
use eframe::egui::{self, *};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

fn selector_viewport_id() -> ViewportId {
    ViewportId::from_hash_of("seletor")
}

fn lupa_viewport_id() -> ViewportId {
    ViewportId::from_hash_of("lupa")
}

/// Magnifier state shared between the panel and the magnifier's own render callback.
struct LupaState {
    app: LupaApp,
    /// Set by the magnifier (ESC / window closed); the panel drops it on its next frame.
    closed: bool,
}

type SharedLupa = Arc<Mutex<LupaState>>;

fn lock(lupa: &SharedLupa) -> MutexGuard<'_, LupaState> {
    lupa.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Short message shown at the bottom of the panel (e.g. "saved", or a save error).
struct Status {
    text: String,
    is_error: bool,
    until: Instant,
}

/// Main control panel. Owns the selector and magnifier windows, which are child
/// viewports of this same process (no extra executables, no IPC files).
pub struct App {
    saved_area: Option<Area>,
    config: AppConfig,
    selector: Option<AreaSelector>,
    /// Rendered as a *deferred* viewport: it has its own repaint loop, so it keeps
    /// updating while the panel is minimized (an immediate viewport would freeze).
    lupa: Option<SharedLupa>,
    status: Option<Status>,
    /// Settings are saved automatically shortly after they stop changing.
    autosave: AutoSave,
}

impl App {
    pub fn new() -> Self {
        let config = config::load_config();
        Self {
            saved_area: config.area,
            autosave: AutoSave::new(&config),
            config,
            selector: None,
            lupa: None,
            status: None,
        }
    }

    fn lupa_position(&self) -> [f32; 2] {
        [self.config.lupa_pos_x, self.config.lupa_pos_y]
    }

    /// Reports the result of a config save in the panel (and the log, for errors).
    fn report_save(&mut self, result: io::Result<()>, success: Option<&str>) {
        let (text, is_error, duration) = match result {
            Ok(()) => match success {
                Some(text) => (text.to_owned(), false, Duration::from_secs(2)),
                None => return,
            },
            Err(err) => {
                log::error!("could not save config: {err}");
                (
                    format!("⚠ Erro ao salvar configurações: {err}"),
                    true,
                    Duration::from_secs(8),
                )
            }
        };
        self.status = Some(Status {
            text,
            is_error,
            until: Instant::now() + duration,
        });
    }

    /// Writes the settings now and reports the result in the panel.
    fn save_now(&mut self, success: Option<&str>) {
        let result = config::save_config(&self.config);
        // Also on error: retrying every frame would only repeat the same error.
        // The next change triggers a new attempt.
        self.autosave.mark_saved(&self.config);
        self.report_save(result, success);
    }

    /// Saves pending slider changes once they have been stable for a moment.
    fn autosave(&mut self, ctx: &egui::Context) {
        match self.autosave.poll(&self.config, Instant::now()) {
            AutoSaveAction::Idle => {}
            AutoSaveAction::Wait(remaining) => ctx.request_repaint_after(remaining),
            AutoSaveAction::SaveNow => self.save_now(Some("✔️ Configurações salvas")),
        }
    }

    /// Opens (or reopens) the magnifier on the saved area, in the saved passthrough mode.
    fn open_lupa(&mut self, ctx: &egui::Context) {
        let Some(area) = self.saved_area else {
            return;
        };
        // The old lupa.exe persisted its position on launch; keep settings in sync on disk.
        self.save_now(None);
        let mut app = LupaApp::new(
            area.to_rect_px(),
            self.config.lupa_zoom,
            self.lupa_position(),
            self.config.lupa_passthrough,
            ctx.pixels_per_point(),
        );
        app.frame_interval = self.config.lupa_frame_interval();
        self.lupa = Some(Arc::new(Mutex::new(LupaState { app, closed: false })));
    }

    /// Flips the saved click-through mode (auto-saved like the sliders). An open
    /// magnifier follows it on the next frame; otherwise it applies on the next open.
    fn toggle_passthrough(&mut self) {
        self.config.lupa_passthrough = !self.config.lupa_passthrough;
    }

    fn show_selector(&mut self, ctx: &egui::Context) {
        let Some(selector) = &mut self.selector else {
            return;
        };
        let builder = AreaSelector::viewport_builder(ui_selector::screen_size(ctx));
        let outcome =
            ctx.show_viewport_immediate(selector_viewport_id(), builder, |ui, _| selector.ui(ui));

        match outcome {
            SelectorOutcome::Pending => {}
            SelectorOutcome::Selected(area) => {
                self.saved_area = Some(area);
                self.config.area = Some(area);
                self.save_now(Some("✔️ Área salva!"));
                self.selector = None;
            }
            SelectorOutcome::Cancelled => self.selector = None,
        }
    }

    /// Drops the magnifier if it closed itself (ESC or window closed).
    fn reap_closed_lupa(&mut self) {
        if self.lupa.as_ref().is_some_and(|lupa| lock(lupa).closed) {
            self.lupa = None;
        }
    }

    fn show_lupa(&mut self, ctx: &egui::Context) {
        let position = self.lupa_position();
        let zoom = self.config.lupa_zoom;
        let passthrough = self.config.lupa_passthrough;
        let frame_interval = self.config.lupa_frame_interval();
        let Some(lupa) = &self.lupa else {
            return;
        };

        let builder = {
            let mut state = lock(lupa);
            // Live updates: slider changes apply to the open magnifier immediately.
            state.app.zoom = zoom;
            state.app.position = position;
            state.app.passthrough = passthrough;
            state.app.frame_interval = frame_interval;
            state.app.viewport_builder()
        };

        let shared = Arc::clone(lupa);
        ctx.show_viewport_deferred(lupa_viewport_id(), builder, move |ui, _| {
            let mut state = lock(&shared);
            if state.closed {
                return; // hidden, waiting for the panel to drop it; stop capturing
            }
            if state.app.ui(ui) == LupaOutcome::Closed {
                state.closed = true;
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
                ui.ctx().request_repaint_of(ViewportId::ROOT);
            }
        });
    }

    fn status_ui(&mut self, ui: &mut Ui) {
        let Some(status) = &self.status else {
            return;
        };
        let now = Instant::now();
        if now >= status.until {
            self.status = None;
            return;
        }
        let color = if status.is_error {
            ui.visuals().error_fg_color
        } else {
            Color32::from_rgb(80, 200, 120)
        };
        ui.colored_label(color, &status.text);
        ui.ctx().request_repaint_after(status.until - now);
    }

    fn panel_ui(&mut self, ui: &mut Ui) {
        let ctx = &ui.ctx().clone();
        let screen = ui_selector::screen_size(ctx);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("🖥️ Área Selector");

                if ui.button("Selecionar Área").clicked() && self.selector.is_none() {
                    // Screenshot is taken now, before the overlay window exists.
                    self.selector = Some(AreaSelector::capture_screen());
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.saved_area.is_some(), Button::new("Iniciar Lupa"))
                        .clicked()
                    {
                        self.open_lupa(ctx);
                    }

                    let passthrough_label = if self.config.lupa_passthrough {
                        "Passthrough: ligado"
                    } else {
                        "Passthrough: desligado"
                    };
                    if ui
                        .button(passthrough_label)
                        .on_hover_text("Alternar Lupa Passthrough: cliques atravessam a lupa")
                        .clicked()
                    {
                        self.toggle_passthrough();
                    }

                    // A passthrough magnifier can't receive clicks or ESC, so it is closed from here.
                    if ui
                        .add_enabled(self.lupa.is_some(), Button::new("Fechar Lupa"))
                        .clicked()
                    {
                        self.lupa = None;
                    }
                });

                ui.separator();

                ui.heading("Configurações da Lupa");
                ui.add(Slider::new(&mut self.config.lupa_zoom, 0.5..=5.0).text("Zoom"));
                ui.add(Slider::new(&mut self.config.lupa_pos_x, 0.0..=screen.x).text("Posição X"));
                ui.add(Slider::new(&mut self.config.lupa_pos_y, 0.0..=screen.y).text("Posição Y"));
                ui.add(Slider::new(&mut self.config.lupa_fps, LUPA_FPS_RANGE).text("FPS"));

                ui.weak("As configurações são salvas automaticamente.");

                ui.separator();

                match self.saved_area {
                    Some(area) => {
                        let (x, y, w, h) = area.to_rect_px();
                        ui.label(format!("Área salva: {w}×{h} px em ({x}, {y})"));
                    }
                    None => {
                        ui.label("Nenhuma área salva ainda.");
                    }
                }

                ui.separator();

                if ui.button("❌ Sair").clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }

                self.status_ui(ui);
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.reap_closed_lupa();
        self.panel_ui(ui);
        self.show_selector(ctx);
        self.show_lupa(ctx);
        self.autosave(ctx);
    }

    /// Saves a change made less than [`config::AUTOSAVE_DELAY`] before closing.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.autosave.has_unsaved_changes(&self.config) {
            if let Err(err) = config::save_config(&self.config) {
                log::error!("could not save config on exit: {err}");
            }
        }
    }
}
