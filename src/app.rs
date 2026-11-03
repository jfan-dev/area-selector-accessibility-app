use area_selector::config::{self, AppConfig};
use area_selector::lupa::{LupaApp, LupaOutcome};
use area_selector::model::Area;
use area_selector::ui_selector::{self, AreaSelector, SelectorOutcome};
use eframe::egui::{self, *};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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

/// Main control panel. Owns the selector and magnifier windows, which are child
/// viewports of this same process (no extra executables, no IPC files).
pub struct App {
    saved_area: Option<Area>,
    config: AppConfig,
    selector: Option<AreaSelector>,
    /// Rendered as a *deferred* viewport: it has its own repaint loop, so it keeps
    /// updating while the panel is minimized (an immediate viewport would freeze).
    lupa: Option<SharedLupa>,
}

impl App {
    pub fn new() -> Self {
        let config = config::load_config();
        Self {
            saved_area: config.area,
            config,
            selector: None,
            lupa: None,
        }
    }

    fn lupa_position(&self) -> [f32; 2] {
        [self.config.lupa_pos_x, self.config.lupa_pos_y]
    }

    /// Opens (or reopens) the magnifier on the saved area.
    fn open_lupa(&mut self, passthrough: bool) {
        let Some(area) = self.saved_area else {
            return;
        };
        // The old lupa.exe persisted its position on launch; keep settings in sync on disk.
        config::save_config(&self.config);
        let app = LupaApp::new(
            area.to_rect_px(),
            self.config.lupa_zoom,
            self.lupa_position(),
            passthrough,
        );
        self.lupa = Some(Arc::new(Mutex::new(LupaState { app, closed: false })));
    }

    /// Flips click-through on the open magnifier, or opens it in passthrough mode.
    fn toggle_passthrough(&mut self) {
        match &self.lupa {
            Some(lupa) => {
                let mut state = lock(lupa);
                state.app.passthrough = !state.app.passthrough;
            }
            None => self.open_lupa(true),
        }
    }

    fn show_selector(&mut self, ctx: &egui::Context) {
        let Some(selector) = &mut self.selector else {
            return;
        };
        let builder = AreaSelector::viewport_builder(ui_selector::screen_size(ctx));
        let outcome =
            ctx.show_viewport_immediate(selector_viewport_id(), builder, |ctx, _| selector.ui(ctx));

        match outcome {
            SelectorOutcome::Pending => {}
            SelectorOutcome::Selected(area) => {
                config::save_area(&area);
                self.saved_area = Some(area);
                self.config.area = Some(area);
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
        let Some(lupa) = &self.lupa else {
            return;
        };

        let builder = {
            let mut state = lock(lupa);
            // Live updates: slider changes apply to the open magnifier immediately.
            state.app.zoom = zoom;
            state.app.position = position;
            state.app.viewport_builder()
        };

        let shared = Arc::clone(lupa);
        ctx.show_viewport_deferred(lupa_viewport_id(), builder, move |ctx, _| {
            let mut state = lock(&shared);
            if state.closed {
                return; // hidden, waiting for the panel to drop it; stop capturing
            }
            if state.app.ui(ctx) == LupaOutcome::Closed {
                state.closed = true;
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                ctx.request_repaint_of(ViewportId::ROOT);
            }
        });
    }

    fn panel_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
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
                        self.open_lupa(false);
                    }

                    if ui.button("Alternar Lupa Passthrough").clicked() {
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
                ui.add(Slider::new(&mut self.config.lupa_pos_x, 0.0..=1920.0).text("Posição X"));
                ui.add(Slider::new(&mut self.config.lupa_pos_y, 0.0..=1080.0).text("Posição Y"));

                if ui.button("💾 Salvar Configurações").clicked() {
                    config::save_config(&self.config);
                    ui.label("✔️ Configurações salvas!");
                }

                ui.separator();

                if let Some(area) = self.saved_area {
                    ui.label(format!(
                        "Área salva: start=({:.1}, {:.1}), end=({:.1}, {:.1})",
                        area.start.x, area.start.y, area.end.x, area.end.y
                    ));
                } else {
                    ui.label("Nenhuma área salva ainda.");
                }

                ui.separator();

                if ui.button("❌ Sair").clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.reap_closed_lupa();
        self.panel_ui(ctx);
        self.show_selector(ctx);
        self.show_lupa(ctx);
    }
}
