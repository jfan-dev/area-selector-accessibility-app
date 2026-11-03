mod panel;

use area_selector::config::{self, AppConfig, Appearance, AutoSave, AutoSaveAction};
use area_selector::hotkeys::{Action, Bindings, HotkeyService};
use area_selector::i18n::{self, Lang, Strings};
use area_selector::lupa::{self, LupaApp, LupaOutcome};
use area_selector::model::Area;
use area_selector::theme;
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

/// Selector state shared with its own render callback. The selector is a *deferred*
/// viewport like the magnifier, so a hotkey can open it while the panel is minimized
/// or hidden (an immediate viewport only repaints together with the panel).
struct SelectorState {
    selector: AreaSelector,
    /// Set by the selector when the user finished or cancelled; read in `App::logic`.
    outcome: Option<SelectorOutcome>,
}

type SharedSelector = Arc<Mutex<SelectorState>>;

fn lock<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Short message shown in the panel footer (e.g. "saved", or a save error).
struct Status {
    text: String,
    is_error: bool,
    until: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Home,
    Hotkeys,
    Appearance,
}

/// Main control panel. Owns the selector and magnifier windows, which are child
/// viewports of this same process (no extra executables, no IPC files).
pub struct App {
    saved_area: Option<Area>,
    config: AppConfig,
    selector: Option<SharedSelector>,
    /// Rendered as a *deferred* viewport: it has its own repaint loop, so it keeps
    /// updating while the panel is minimized (an immediate viewport would freeze).
    lupa: Option<SharedLupa>,
    status: Option<Status>,
    /// Settings are saved automatically shortly after they stop changing.
    autosave: AutoSave,
    tab: Tab,
    /// Windows display language, used when no language is configured.
    system_lang: Lang,
    /// Appearance last passed to [`theme::apply`], to re-apply only on change.
    applied_appearance: Option<Appearance>,
    /// Keyboard focus last frame, to scroll newly focused controls into view.
    last_focus: Option<Id>,
    /// System-wide hotkeys (`None` in tests, or if Windows refused to create them).
    hotkeys: Option<HotkeyService>,
    /// Bindings currently registered with Windows (`None` = must re-register).
    registered_bindings: Option<Bindings>,
    /// Action whose new combination is being recorded in the Hotkeys tab.
    recording: Option<Action>,
    /// How far the panel content sticks out past the window edge (layout tests).
    #[cfg(test)]
    layout_overflow: f32,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let hotkeys = HotkeyService::new(ctx);
        Self::with_config(ctx, config::load_config(), Lang::system(), Some(hotkeys))
    }

    fn with_config(
        ctx: &egui::Context,
        config: AppConfig,
        system_lang: Lang,
        hotkeys: Option<HotkeyService>,
    ) -> Self {
        let mut app = Self {
            saved_area: config.area,
            autosave: AutoSave::new(&config),
            config,
            selector: None,
            lupa: None,
            status: None,
            tab: Tab::Home,
            system_lang,
            applied_appearance: None,
            last_focus: None,
            hotkeys,
            registered_bindings: None,
            recording: None,
            #[cfg(test)]
            layout_overflow: 0.0,
        };
        app.apply_theme(ctx);
        app.sync_hotkeys();
        app
    }

    /// Registers the configured hotkeys when they changed (not while recording a new
    /// combination: then they stay suspended so they don't swallow the keys).
    fn sync_hotkeys(&mut self) {
        let Some(service) = &mut self.hotkeys else {
            return;
        };
        if self.recording.is_some() {
            if self.registered_bindings.take().is_some() {
                service.suspend();
            }
            return;
        }
        if self.registered_bindings.as_ref() != Some(&self.config.hotkeys) {
            service.apply(&self.config.hotkeys);
            self.registered_bindings = Some(self.config.hotkeys.clone());
        }
    }

    /// Pretty name of an action's hotkey, if it has one.
    fn hotkey_label(&self, action: Action) -> Option<String> {
        area_selector::hotkeys::pretty(self.config.hotkeys.get(&action)?)
    }

    /// Runs a global hotkey action. Called from [`eframe::App::logic`], so it also works
    /// while the panel is minimized or hidden.
    fn run_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::TogglePassthrough => {
                self.config.lupa_passthrough = !self.config.lupa_passthrough;
            }
            Action::ToggleLupa => {
                if self.lupa.is_some() {
                    self.close_lupa();
                } else {
                    self.open_lupa(ctx);
                }
            }
            Action::SelectArea => self.open_selector(),
            Action::ZoomIn => self.config.lupa_zoom = lupa::step_zoom(self.config.lupa_zoom, 1),
            Action::ZoomOut => self.config.lupa_zoom = lupa::step_zoom(self.config.lupa_zoom, -1),
            Action::ShowPanel => {
                ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
        }
        // Run one full pass even if the panel is minimized/hidden, so new windows are
        // created and setting changes reach the magnifier.
        ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
    }

    fn strings(&self) -> &'static Strings {
        i18n::tr(self.config.language.unwrap_or(self.system_lang))
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.applied_appearance.as_ref() != Some(&self.config.appearance) {
            theme::apply(ctx, &self.config.appearance);
            self.applied_appearance = Some(self.config.appearance.clone());
        }
    }

    fn lupa_position(&self) -> [f32; 2] {
        [self.config.lupa_pos_x, self.config.lupa_pos_y]
    }

    /// Reports the result of a config save in the panel (and the log, for errors).
    fn report_save(&mut self, result: io::Result<()>, success: Option<&'static str>) {
        let (text, is_error, duration) = match result {
            Ok(()) => match success {
                Some(text) => (text.to_owned(), false, Duration::from_secs(2)),
                None => return,
            },
            Err(err) => {
                log::error!("could not save config: {err}");
                let text = (self.strings().save_error)(&err.to_string());
                (text, true, Duration::from_secs(8))
            }
        };
        self.status = Some(Status {
            text,
            is_error,
            until: Instant::now() + duration,
        });
    }

    /// Writes the settings now and reports the result in the panel.
    fn save_now(&mut self, success: Option<&'static str>) {
        let result = config::save_config(&self.config);
        // Also on error: retrying every frame would only repeat the same error.
        // The next change triggers a new attempt.
        self.autosave.mark_saved(&self.config);
        self.report_save(result, success);
    }

    /// Saves pending changes once they have been stable for a moment.
    fn autosave(&mut self, ctx: &egui::Context) {
        match self.autosave.poll(&self.config, Instant::now()) {
            AutoSaveAction::Idle => {}
            AutoSaveAction::Wait(remaining) => ctx.request_repaint_after(remaining),
            AutoSaveAction::SaveNow => self.save_now(Some(self.strings().saved_settings)),
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

    fn close_lupa(&mut self) {
        self.lupa = None;
    }

    fn open_selector(&mut self) {
        if self.selector.is_none() {
            // Screenshot is taken now, before the overlay window exists.
            let strings = self.strings();
            let selector = AreaSelector::capture_screen().with_strings(strings);
            self.selector = Some(Arc::new(Mutex::new(SelectorState {
                selector,
                outcome: None,
            })));
        }
    }

    fn show_selector(&mut self, ctx: &egui::Context) {
        let Some(selector) = &self.selector else {
            return;
        };
        let builder = AreaSelector::viewport_builder(ui_selector::screen_size(ctx));
        let shared = Arc::clone(selector);
        ctx.show_viewport_deferred(selector_viewport_id(), builder, move |ui, _| {
            let mut state = lock(&shared);
            if state.outcome.is_some() {
                return; // finished, waiting for the panel to close it
            }
            let outcome = state.selector.ui(ui);
            if outcome != SelectorOutcome::Pending {
                state.outcome = Some(outcome);
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
                ui.ctx().request_repaint_of(ViewportId::ROOT);
            }
        });
    }

    /// Applies a finished selection (or cancellation) and closes the selector.
    fn reap_selector(&mut self) {
        let outcome = match &self.selector {
            Some(selector) => lock(selector).outcome,
            None => return,
        };
        match outcome {
            None | Some(SelectorOutcome::Pending) => {}
            Some(SelectorOutcome::Selected(area)) => {
                self.saved_area = Some(area);
                self.config.area = Some(area);
                // An open magnifier switches to the new area right away.
                if let Some(lupa) = &self.lupa {
                    lock(lupa).app.set_area(area.to_rect_px());
                }
                self.save_now(Some(self.strings().saved_area));
                self.selector = None;
            }
            Some(SelectorOutcome::Cancelled) => self.selector = None,
        }
    }

    /// Drops the magnifier if it closed itself (ESC or window closed).
    fn reap_closed_lupa(&mut self) {
        if self.lupa.as_ref().is_some_and(|lupa| lock(lupa).closed) {
            self.lupa = None;
        }
    }

    /// Changes made on the magnifier itself (drag, wheel, keys) become settings (and are
    /// auto-saved). The magnifier already applied them to its own window.
    fn pull_lupa_changes(&mut self) {
        let Some(lupa) = &self.lupa else {
            return;
        };
        let changes = lock(lupa).app.take_user_changes();
        if let Some(zoom) = changes.zoom {
            self.config.lupa_zoom = zoom;
        }
        if let Some([x, y]) = changes.position {
            self.config.lupa_pos_x = x;
            self.config.lupa_pos_y = y;
        }
        if let Some(passthrough) = changes.passthrough {
            self.config.lupa_passthrough = passthrough;
        }
    }

    fn show_lupa(&mut self, ctx: &egui::Context) {
        self.pull_lupa_changes();
        let strings = self.strings();
        let Some(lupa) = &self.lupa else {
            return;
        };

        let builder = {
            let mut state = lock(lupa);
            let app = &mut state.app;

            // Panel settings apply to the open magnifier immediately.
            let config = &self.config;
            app.zoom = config.lupa_zoom;
            app.set_position([config.lupa_pos_x, config.lupa_pos_y]);
            app.passthrough = config.lupa_passthrough;
            app.frame_interval = config.lupa_frame_interval();
            app.border_colors = config.appearance.border_preset.colors();
            app.border_width = config.appearance.border_width;
            app.show_mode_badge = config.appearance.show_mode_badge;
            app.strings = strings;
            app.viewport_builder()
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
}

impl eframe::App for App {
    /// Runs before every `ui` pass and also while the panel is minimized or hidden.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let actions = self
            .hotkeys
            .as_mut()
            .map(HotkeyService::poll)
            .unwrap_or_default();
        for action in actions {
            self.run_action(ctx, action);
        }
        self.reap_selector();
        self.reap_closed_lupa();
        self.sync_hotkeys();
        self.autosave(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.apply_theme(ctx);
        self.panel_ui(ui);
        self.show_selector(ctx);
        self.show_lupa(ctx);
    }

    /// Saves a change made less than [`config::AUTOSAVE_DELAY`] before closing.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Include changes made on the magnifier while the panel wasn't drawing.
        self.pull_lupa_changes();
        if self.autosave.has_unsaved_changes(&self.config) {
            if let Err(err) = config::save_config(&self.config) {
                log::error!("could not save config on exit: {err}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use area_selector::model::SerializablePos;

    /// Renders the real panel headlessly and returns how far its content sticks out
    /// past the window's right edge (0 = everything fits).
    fn overflow(lang: Lang, width: f32, ui_scale: f32, tab: Tab) -> f32 {
        let ctx = egui::Context::default();
        let mut config = AppConfig::default();
        config.appearance.ui_scale = ui_scale;
        config.area = Some(Area {
            start: SerializablePos { x: 10.0, y: 20.0 },
            end: SerializablePos { x: 304.0, y: 234.0 },
        });
        let mut app = App::with_config(&ctx, config, lang, None);
        app.tab = tab;
        let input = || RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 720.0))),
            ..Default::default()
        };
        // A few passes so egui's remembered sizes settle.
        for _ in 0..4 {
            let mut output = ctx.run_ui(input(), |ui| app.panel_ui(ui));
            output.textures_delta.clear(); // no GPU here: discard texture uploads
        }
        app.layout_overflow
    }

    #[test]
    fn panel_fits_every_window_width_and_text_size() {
        for lang in Lang::ALL {
            for width in [320.0, 400.0, 480.0, 600.0, 900.0] {
                for scale in [1.0, 1.5, 2.0] {
                    for tab in [Tab::Home, Tab::Hotkeys, Tab::Appearance] {
                        let over = overflow(lang, width, scale, tab);
                        assert!(
                            over <= 1.0,
                            "{lang:?} {tab:?} at {width} px, {:.0}%: content is {over:.0} px too wide",
                            scale * 100.0
                        );
                    }
                }
            }
        }
    }
}
