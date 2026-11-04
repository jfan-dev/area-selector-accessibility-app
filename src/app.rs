mod magnifier_window;
mod panel;

use area_selector::config::{
    self, AppConfig, Appearance, AutoSave, AutoSaveAction, MagnifierSettings,
};
use area_selector::hotkeys::{Action, Bindings, HotkeyService};
use area_selector::i18n::{self, Lang, Strings};
use area_selector::lupa::{self, LupaApp, LupaOutcome};
use area_selector::theme;
use area_selector::tray::{Tray, TrayCommand, TrayLabels, TrayMagnifier, TrayState};
use area_selector::ui_selector::{self, AreaSelector, SelectorOutcome};
use eframe::egui::{self, *};
use magnifier_window::{MagnifierRequest, WindowState};
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

fn selector_viewport_id() -> ViewportId {
    ViewportId::from_hash_of("seletor")
}

fn lupa_viewport_id(magnifier_id: u32) -> ViewportId {
    ViewportId::from_hash_of(("lupa", magnifier_id))
}

/// Magnifier window state shared between the app and the window's own render callback.
struct LupaState {
    app: LupaApp,
    /// Set by the magnifier (ESC / window closed); the app drops it on its next frame.
    closed: bool,
}

type SharedLupa = Arc<Mutex<LupaState>>;
type SharedSettings = Arc<Mutex<MagnifierSettings>>;
type SharedWindow = Arc<Mutex<WindowState>>;

/// One magnifier: its settings (the single source of truth, shared with its magnifier
/// and settings windows) and the windows that are open for it.
struct MagnifierEntry {
    settings: SharedSettings,
    /// The magnifier window (a *deferred* viewport with its own repaint loop).
    lupa: Option<SharedLupa>,
    /// The settings window, when open.
    window: Option<SharedWindow>,
}

impl MagnifierEntry {
    fn new(settings: MagnifierSettings) -> Self {
        Self {
            settings: Arc::new(Mutex::new(settings)),
            lupa: None,
            window: None,
        }
    }

    fn id(&self) -> u32 {
        lock(&self.settings).id
    }

    /// Changes made on the magnifier itself (drag, wheel, keys) become settings (and are
    /// auto-saved). The magnifier already applied them to its own window.
    fn pull_lupa_changes(&self) {
        let Some(lupa) = &self.lupa else {
            return;
        };
        let changes = lock(lupa).app.take_user_changes();
        let mut settings = lock(&self.settings);
        if let Some(zoom) = changes.zoom {
            settings.zoom = zoom;
        }
        if let Some([x, y]) = changes.position {
            settings.pos_x = x;
            settings.pos_y = y;
        }
        if let Some(passthrough) = changes.passthrough {
            settings.passthrough = passthrough;
        }
    }
}

/// Selector state shared with its own render callback. The selector is a *deferred*
/// viewport like the magnifiers, so a hotkey can open it while the panel is minimized
/// or hidden (an immediate viewport only repaints together with the panel).
struct SelectorState {
    selector: AreaSelector,
    /// Set by the selector when the user finished or cancelled; read in `App::logic`.
    outcome: Option<SelectorOutcome>,
}

type SharedSelector = Arc<Mutex<SelectorState>>;

/// What a finished selection is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorTarget {
    /// Create a new magnifier with the selected area.
    New,
    /// Change the area of an existing magnifier.
    Magnifier(u32),
}

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

/// Where the text-size Apply button was when clicked, so the panel can scroll it back
/// to the same screen position once the new size is laid out.
#[derive(Clone, Copy)]
struct ScaleAnchor {
    /// Screen top of the button when clicked.
    top: f32,
    /// Screen top of the button this frame (its id changes when the layout reflows).
    now: Option<f32>,
    /// A scroll was just requested; skip a frame so it shows before measuring again.
    settling: bool,
    /// Body font size when clicked (a different size = new style is laid out).
    body_size: f32,
    /// Frames left to wait for the new style and scroll (a safety limit).
    frames_left: u8,
}

/// Main control panel. Owns the magnifiers, their settings windows and the selector,
/// which are all child viewports of this same process.
pub struct App {
    /// Panel-wide settings. `config.magnifiers` is a copy of the entries' settings,
    /// refreshed by [`App::sync_config`] before saving.
    config: AppConfig,
    magnifiers: Vec<MagnifierEntry>,
    /// Settings windows send their requests here; handled in `App::logic`.
    requests: (Sender<MagnifierRequest>, Receiver<MagnifierRequest>),
    selector: Option<SharedSelector>,
    selector_target: SelectorTarget,
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
    /// Text size chosen with − / + but not applied yet (shown only in the preview).
    pending_ui_scale: Option<f32>,
    /// Set by the text-size Apply button to scroll it back under the mouse.
    scale_anchor: Option<ScaleAnchor>,
    /// System-wide hotkeys (`None` in tests, or if Windows refused to create them).
    hotkeys: Option<HotkeyService>,
    /// Bindings currently registered with Windows (`None` = must re-register).
    registered_bindings: Option<Bindings>,
    /// Action whose new combination is being recorded in the Hotkeys tab.
    recording: Option<Action>,
    /// The last key pressed while recording had no Ctrl/Alt (shows a message).
    recording_invalid: bool,
    /// App icon for the panel header (uploaded on first use).
    icon_texture: Option<TextureHandle>,
    /// System tray icon (`None` in tests, or if Windows refused to create it).
    tray: Option<Tray>,
    /// Set by "Quit": the next close request really closes the app.
    quitting: bool,
    /// One-time explanation shown on the first close-to-tray.
    show_tray_notice: bool,
    tray_notice_dont_show: bool,
    /// First-run guide (also reopened with the "?" button) and its current step.
    show_onboarding: bool,
    /// The panel is minimized: magnifiers hide their mode indicator.
    panel_minimized: bool,
    /// The panel was hidden to the tray (egui doesn't report window visibility).
    panel_in_tray: bool,
    onboarding_step: usize,
    /// How far the panel content sticks out past the window edge (layout tests).
    #[cfg(test)]
    layout_overflow: f32,
    /// Height taken by the first side-by-side card row and the shared card height (tests).
    #[cfg(test)]
    card_row: (f32, f32),
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let hotkeys = HotkeyService::new(ctx);
        let tray = Tray::new(ctx);
        Self::with_config(
            ctx,
            config::load_config(),
            Lang::system(),
            Some(hotkeys),
            tray,
        )
    }

    fn with_config(
        ctx: &egui::Context,
        config: AppConfig,
        system_lang: Lang,
        hotkeys: Option<HotkeyService>,
        tray: Option<Tray>,
    ) -> Self {
        let show_onboarding = !config.onboarding_done;
        let magnifiers = config
            .magnifiers
            .iter()
            .cloned()
            .map(MagnifierEntry::new)
            .collect();
        let mut app = Self {
            autosave: AutoSave::new(&config),
            config,
            magnifiers,
            requests: mpsc::channel(),
            selector: None,
            selector_target: SelectorTarget::New,
            status: None,
            tab: Tab::Home,
            system_lang,
            applied_appearance: None,
            last_focus: None,
            pending_ui_scale: None,
            scale_anchor: None,
            hotkeys,
            registered_bindings: None,
            recording: None,
            recording_invalid: false,
            icon_texture: None,
            show_onboarding,
            panel_minimized: false,
            panel_in_tray: false,
            tray,
            quitting: false,
            show_tray_notice: false,
            tray_notice_dont_show: false,
            onboarding_step: 0,
            #[cfg(test)]
            layout_overflow: 0.0,
            #[cfg(test)]
            card_row: (0.0, 0.0),
        };
        app.apply_theme(ctx);
        app.sync_hotkeys();
        // Magnifiers that were open when the app closed open again.
        for index in 0..app.magnifiers.len() {
            let reopen = {
                let settings = lock(&app.magnifiers[index].settings);
                settings.open && settings.area.is_some()
            };
            if reopen {
                app.open_magnifier(ctx, index);
            }
        }
        app
    }

    // ----------------------------------------------------------------------------
    // Magnifiers

    fn index_of(&self, id: u32) -> Option<usize> {
        self.magnifiers.iter().position(|entry| entry.id() == id)
    }

    fn selected_index(&self) -> Option<usize> {
        self.config
            .selected
            .and_then(|id| self.index_of(id))
            .or(if self.magnifiers.is_empty() {
                None
            } else {
                Some(0)
            })
    }

    fn can_add_magnifier(&self) -> bool {
        self.magnifiers.len() < config::MAX_MAGNIFIERS
    }

    fn open_magnifier(&mut self, ctx: &egui::Context, index: usize) {
        let entry = &mut self.magnifiers[index];
        let mut settings = lock(&entry.settings);
        let Some(area) = settings.area else {
            return;
        };
        let mut app = LupaApp::new(
            area.to_rect_px(),
            settings.zoom,
            [settings.pos_x, settings.pos_y],
            settings.passthrough,
            ctx.pixels_per_point(),
        );
        app.title = LupaApp::title_for(settings.id);
        settings.open = true;
        drop(settings);
        entry.lupa = Some(Arc::new(Mutex::new(LupaState { app, closed: false })));
    }

    fn close_magnifier(&mut self, index: usize) {
        let entry = &mut self.magnifiers[index];
        entry.lupa = None;
        lock(&entry.settings).open = false;
    }

    fn toggle_magnifier(&mut self, ctx: &egui::Context, index: usize) {
        if self.magnifiers[index].lupa.is_some() {
            self.close_magnifier(index);
        } else {
            self.open_magnifier(ctx, index);
        }
    }

    fn open_settings_window(&mut self, index: usize) {
        let strings = self.strings();
        let entry = &mut self.magnifiers[index];
        if entry.window.is_none() {
            entry.window = Some(Arc::new(Mutex::new(WindowState::new(strings))));
        }
    }

    /// Opens the selector to create a new magnifier.
    fn add_magnifier(&mut self) {
        if self.can_add_magnifier() {
            self.open_selector(SelectorTarget::New);
        }
    }

    fn delete_magnifier(&mut self, id: u32) {
        if let Some(index) = self.index_of(id) {
            self.magnifiers.remove(index);
            if self.config.selected == Some(id) {
                self.config.selected = self.magnifiers.first().map(MagnifierEntry::id);
            }
        }
    }

    fn select_magnifier(&mut self, id: u32) {
        self.config.selected = Some(id);
    }

    /// Requests from the settings windows.
    fn handle_requests(&mut self, ctx: &egui::Context) {
        let requests: Vec<_> = self.requests.1.try_iter().collect();
        for request in requests {
            match request {
                MagnifierRequest::SelectArea(id) => {
                    self.open_selector(SelectorTarget::Magnifier(id));
                }
                MagnifierRequest::ToggleOpen(id) => {
                    if let Some(index) = self.index_of(id) {
                        self.toggle_magnifier(ctx, index);
                    }
                }
                MagnifierRequest::Delete(id) => self.delete_magnifier(id),
            }
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
    }

    /// Copies the entries' settings into `config.magnifiers` (for saving).
    fn sync_config(&mut self) {
        self.config.magnifiers = self
            .magnifiers
            .iter()
            .map(|entry| lock(&entry.settings).clone())
            .collect();
        let selected_exists = self
            .config
            .selected
            .is_some_and(|id| self.config.magnifiers.iter().any(|m| m.id == id));
        if !selected_exists {
            self.config.selected = self.config.magnifiers.first().map(|m| m.id);
        }
    }

    /// Drops magnifier windows and settings windows that closed themselves.
    fn reap_closed_windows(&mut self) {
        for entry in &mut self.magnifiers {
            if entry.lupa.as_ref().is_some_and(|lupa| lock(lupa).closed) {
                entry.lupa = None;
                lock(&entry.settings).open = false;
            }
            if entry.window.as_ref().is_some_and(|w| lock(w).closed) {
                entry.window = None;
            }
        }
    }

    /// While the panel is minimized (the player is in the game) the magnifiers hide their
    /// mode indicator; it comes back when the panel is restored. The saved setting is not
    /// changed. Runs in `logic`, because `ui` doesn't run while minimized.
    fn update_badges_for_minimized_panel(&mut self, ctx: &egui::Context) {
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized == self.panel_minimized {
            return;
        }
        self.panel_minimized = minimized;
        for entry in &self.magnifiers {
            if let Some(lupa) = &entry.lupa {
                let show = lock(&entry.settings).show_badge && !minimized;
                lock(lupa).app.show_mode_badge = show;
            }
        }
    }

    fn show_magnifiers(&mut self, ctx: &egui::Context) {
        let strings = self.strings();
        for entry in &self.magnifiers {
            entry.pull_lupa_changes();
            let settings = lock(&entry.settings).clone();
            if let Some(lupa) = &entry.lupa {
                show_lupa(ctx, lupa, &settings, strings, self.panel_minimized);
            }
            if let Some(window) = &entry.window {
                {
                    let mut state = lock(window);
                    state.lupa_open = entry.lupa.is_some();
                    state.strings = strings;
                }
                show_settings_window(ctx, &settings, entry, window, &self.requests.0);
            }
        }
    }

    // ----------------------------------------------------------------------------
    // Selector

    fn open_selector(&mut self, target: SelectorTarget) {
        if self.selector.is_none() {
            // Screenshot is taken now, before the overlay window exists.
            let strings = self.strings();
            let selector = AreaSelector::capture_screen().with_strings(strings);
            self.selector_target = target;
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
                return; // finished, waiting for the app to close it
            }
            let outcome = state.selector.ui(ui);
            if outcome != SelectorOutcome::Pending {
                state.outcome = Some(outcome);
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
                ui.ctx().request_repaint_of(ViewportId::ROOT);
            }
        });
    }

    /// Applies a finished selection: a new magnifier, or a new area for an existing one.
    fn reap_selector(&mut self, ctx: &egui::Context) {
        let outcome = match &self.selector {
            Some(selector) => lock(selector).outcome,
            None => return,
        };
        let area = match outcome {
            None | Some(SelectorOutcome::Pending) => return,
            Some(SelectorOutcome::Cancelled) => {
                self.selector = None;
                return;
            }
            Some(SelectorOutcome::Selected(area)) => area,
        };
        self.selector = None;
        match self.selector_target {
            SelectorTarget::New => {
                if !self.can_add_magnifier() {
                    return;
                }
                self.sync_config();
                let id = self.config.next_magnifier_id();
                let name = format!("{} {id}", self.strings().magnifier_default_name);
                let [pos_x, pos_y] = config::cascade_position(self.magnifiers.len());
                let settings = MagnifierSettings {
                    area: Some(area),
                    pos_x,
                    pos_y,
                    ..MagnifierSettings::new(id, name)
                };
                self.magnifiers.push(MagnifierEntry::new(settings));
                self.select_magnifier(id);
                let index = self.magnifiers.len() - 1;
                self.open_magnifier(ctx, index);
            }
            SelectorTarget::Magnifier(id) => {
                if let Some(index) = self.index_of(id) {
                    let entry = &self.magnifiers[index];
                    lock(&entry.settings).area = Some(area);
                    // An open magnifier switches to the new area right away.
                    if let Some(lupa) = &entry.lupa {
                        lock(lupa).app.set_area(area.to_rect_px());
                    }
                }
            }
        }
        self.save_now(Some(self.strings().saved_area));
    }

    // ----------------------------------------------------------------------------
    // Hotkeys, tray, close

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

    /// Passthrough for all magnifiers (see [`config::all_passthrough_target`]).
    fn toggle_all_passthrough(&mut self) {
        let states: Vec<bool> = self
            .magnifiers
            .iter()
            .map(|entry| lock(&entry.settings).passthrough)
            .collect();
        let target = config::all_passthrough_target(&states);
        for entry in &self.magnifiers {
            lock(&entry.settings).passthrough = target;
        }
    }

    /// Show/hide all: if any magnifier is open, close all; else open all with an area.
    fn toggle_all_magnifiers(&mut self, ctx: &egui::Context) {
        let any_open = self.magnifiers.iter().any(|entry| entry.lupa.is_some());
        for index in 0..self.magnifiers.len() {
            if any_open {
                self.close_magnifier(index);
            } else {
                self.open_magnifier(ctx, index);
            }
        }
    }

    /// Runs a global hotkey action. Called from [`eframe::App::logic`], so it also works
    /// while the panel is minimized or hidden. Passthrough and show/hide act on all
    /// magnifiers; zoom and select area on the selected one.
    fn run_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::TogglePassthrough => self.toggle_all_passthrough(),
            Action::ToggleLupa => self.toggle_all_magnifiers(ctx),
            Action::SelectArea => match self.selected_index() {
                Some(index) => {
                    let id = self.magnifiers[index].id();
                    self.open_selector(SelectorTarget::Magnifier(id));
                }
                None => self.add_magnifier(),
            },
            Action::ZoomIn | Action::ZoomOut => {
                let steps = if action == Action::ZoomIn { 1 } else { -1 };
                if let Some(index) = self.selected_index() {
                    let mut settings = lock(&self.magnifiers[index].settings);
                    settings.zoom = lupa::step_zoom(settings.zoom, steps);
                }
            }
            Action::CycleSelection => {
                let ids: Vec<u32> = self.magnifiers.iter().map(MagnifierEntry::id).collect();
                if let Some(id) = config::next_selection(&ids, self.config.selected) {
                    self.select_magnifier(id);
                    if let Some(lupa) = self
                        .index_of(id)
                        .and_then(|i| self.magnifiers[i].lupa.clone())
                    {
                        lock(&lupa).app.flash();
                    }
                }
            }
            Action::ShowPanel => self.toggle_panel(ctx),
        }
        // Run one full pass even if the panel is minimized/hidden, so new windows are
        // created and setting changes reach the magnifiers.
        ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
    }

    /// Closes the app for real (also from the tray, while the panel is hidden).
    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    fn hide_to_tray(&mut self, ctx: &egui::Context) {
        self.panel_in_tray = true;
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    }

    /// Brings the panel back from the tray or the taskbar, in front of the game.
    fn show_panel(&mut self, ctx: &egui::Context) {
        self.panel_in_tray = false;
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    /// Show/hide hotkey: a panel that is on screen and focused gets minimized (the
    /// magnifiers then hide their mode indicator); otherwise — minimized, in the tray or
    /// behind the game — it is brought to the front.
    fn toggle_panel(&mut self, ctx: &egui::Context) {
        let (minimized, focused) = ctx.input(|i| {
            let viewport = i.viewport();
            (
                viewport.minimized.unwrap_or(false),
                viewport.focused.unwrap_or(false),
            )
        });
        if !self.panel_in_tray && !minimized && focused {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        } else {
            self.show_panel(ctx);
        }
    }

    /// Closing the panel hides it to the tray (when enabled and a tray icon exists);
    /// the first time, a short explanation is shown instead.
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        let close = ctx.input(|i| i.viewport().close_requested());
        if !close || self.quitting || self.tray.is_none() || !self.config.close_to_tray {
            return;
        }
        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        if self.config.tray_notice_shown {
            self.hide_to_tray(ctx);
        } else {
            self.show_tray_notice = true;
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
    }

    fn handle_tray(&mut self, ctx: &egui::Context) {
        let commands = self.tray.as_mut().map(Tray::poll).unwrap_or_default();
        for command in commands {
            match command {
                TrayCommand::ShowPanel => {
                    self.show_panel(ctx);
                    ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
                }
                TrayCommand::PassthroughAll => self.run_action(ctx, Action::TogglePassthrough),
                TrayCommand::ToggleMagnifier(id) => {
                    if let Some(index) = self.index_of(id) {
                        self.toggle_magnifier(ctx, index);
                        ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
                    }
                }
                TrayCommand::Quit => self.quit(ctx),
            }
        }

        let s = self.strings();
        let item = |text: &str, action: Action| match self.hotkey_label(action) {
            Some(hotkey) => format!("{text}\t{hotkey}"),
            None => text.to_owned(),
        };
        let labels = TrayLabels {
            tooltip: s.app_title.to_owned(),
            show_panel: item(s.tray_show, Action::ShowPanel),
            passthrough_all: item(s.tray_passthrough_all, Action::TogglePassthrough),
            quit: s.tray_quit.to_owned(),
        };
        let magnifiers: Vec<TrayMagnifier> = self
            .magnifiers
            .iter()
            .map(|entry| {
                let settings = lock(&entry.settings);
                TrayMagnifier {
                    id: settings.id,
                    name: settings.name.clone(),
                    open: entry.lupa.is_some(),
                    has_area: settings.area.is_some(),
                }
            })
            .collect();
        let all_passthrough = !self.magnifiers.is_empty()
            && self
                .magnifiers
                .iter()
                .all(|entry| lock(&entry.settings).passthrough);
        let state = TrayState {
            magnifiers,
            all_passthrough,
        };
        if let Some(tray) = &mut self.tray {
            tray.update(&labels, &state);
        }
    }

    /// The app icon as a texture for the panel header.
    fn icon_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        if self.icon_texture.is_none() {
            let icon = area_selector::icon::app_icon()?;
            let size = [icon.width as usize, icon.height as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, &icon.rgba);
            self.icon_texture = Some(ctx.load_texture("app_icon", image, TextureOptions::LINEAR));
        }
        self.icon_texture.clone()
    }

    /// Pretty name of an action's hotkey, if it has one.
    fn hotkey_label(&self, action: Action) -> Option<String> {
        area_selector::hotkeys::pretty(self.config.hotkeys.get(&action)?)
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

    // ----------------------------------------------------------------------------
    // Saving

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
        self.sync_config();
        let result = config::save_config(&self.config);
        // Also on error: retrying every frame would only repeat the same error.
        // The next change triggers a new attempt.
        self.autosave.mark_saved(&self.config);
        self.report_save(result, success);
    }

    /// Saves pending changes once they have been stable for a moment.
    fn autosave(&mut self, ctx: &egui::Context) {
        self.sync_config();
        match self.autosave.poll(&self.config, Instant::now()) {
            AutoSaveAction::Idle => {}
            AutoSaveAction::Wait(remaining) => ctx.request_repaint_after(remaining),
            AutoSaveAction::SaveNow => self.save_now(Some(self.strings().saved_settings)),
        }
    }
}

/// Shows (or keeps) one magnifier window, applying its current settings.
fn show_lupa(
    ctx: &egui::Context,
    lupa: &SharedLupa,
    settings: &MagnifierSettings,
    strings: &'static Strings,
    panel_minimized: bool,
) {
    let builder = {
        let mut state = lock(lupa);
        let app = &mut state.app;
        app.zoom = settings.zoom;
        app.set_position([settings.pos_x, settings.pos_y]);
        app.passthrough = settings.passthrough;
        app.frame_interval = settings.frame_interval();
        app.border_colors = settings.border_preset.colors();
        app.border_width = settings.border_width;
        app.show_mode_badge = settings.show_badge && !panel_minimized;
        app.name = settings.name.clone();
        app.strings = strings;
        app.viewport_builder()
    };
    let shared = Arc::clone(lupa);
    ctx.show_viewport_deferred(lupa_viewport_id(settings.id), builder, move |ui, _| {
        let mut state = lock(&shared);
        if state.closed {
            return; // hidden, waiting for the app to drop it; stop capturing
        }
        if state.app.ui(ui) == LupaOutcome::Closed {
            state.closed = true;
            ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
            ui.ctx().request_repaint_of(ViewportId::ROOT);
        }
    });
}

/// Shows (or keeps) one magnifier's settings window.
fn show_settings_window(
    ctx: &egui::Context,
    settings: &MagnifierSettings,
    entry: &MagnifierEntry,
    window: &SharedWindow,
    requests: &Sender<MagnifierRequest>,
) {
    let title = (lock(window).strings.settings_window_title)(&settings.name);
    let builder = magnifier_window::viewport_builder(title);
    let shared_settings = Arc::clone(&entry.settings);
    let shared_window = Arc::clone(window);
    let requests = requests.clone();
    ctx.show_viewport_deferred(
        magnifier_window::viewport_id(settings.id),
        builder,
        move |ui, _| {
            let mut state = lock(&shared_window);
            if state.closed {
                return;
            }
            let mut settings = lock(&shared_settings);
            let before = settings.clone();
            let new_requests = magnifier_window::window_ui(ui, &mut settings, &mut state);
            // The panel's pass is what pushes settings to the magnifier window (size,
            // position, passthrough, title) and saves them, so wake it on any change —
            // also while it is minimized or hidden in the tray.
            let wake_panel = state.closed || !new_requests.is_empty() || *settings != before;
            for request in new_requests {
                let _ = requests.send(request);
            }
            if wake_panel {
                let ctx = ui.ctx();
                ctx.send_viewport_cmd_to(
                    ViewportId::ROOT,
                    ViewportCommand::RequestPaintWhileHidden,
                );
                ctx.request_repaint_of(ViewportId::ROOT);
            }
        },
    );
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
        self.handle_tray(ctx);
        self.handle_close_request(ctx);
        self.handle_requests(ctx);
        self.update_badges_for_minimized_panel(ctx);
        self.reap_selector(ctx);
        self.reap_closed_windows();
        self.sync_hotkeys();
        self.autosave(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.apply_theme(ctx);
        self.panel_ui(ui);
        self.show_selector(ctx);
        self.show_magnifiers(ctx);
    }

    /// Saves a change made less than [`config::AUTOSAVE_DELAY`] before closing.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Include changes made on the magnifiers while the panel wasn't drawing.
        for entry in &self.magnifiers {
            entry.pull_lupa_changes();
        }
        self.sync_config();
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
    use area_selector::config::{BorderPreset, HomeView, UI_SCALE_RANGE};
    use area_selector::model::{Area, SerializablePos};
    use egui_kittest::kittest::Queryable;

    fn area() -> Area {
        Area {
            start: SerializablePos { x: 10.0, y: 20.0 },
            end: SerializablePos { x: 304.0, y: 234.0 },
        }
    }

    /// `n` magnifiers with varied names and settings (some without an area).
    fn sample_magnifiers(n: usize) -> Vec<MagnifierSettings> {
        let names = [
            "Minimapa",
            "Vida",
            "Chat do jogo com um nome bem comprido",
            "Mana",
        ];
        (0..n)
            .map(|i| MagnifierSettings {
                area: (i % 3 != 2).then(area),
                passthrough: i % 2 == 1,
                border_preset: BorderPreset::ALL[i % 3],
                ..MagnifierSettings::new(i as u32 + 1, names[i % names.len()].to_owned())
            })
            .collect()
    }

    fn test_config(magnifiers: usize) -> AppConfig {
        AppConfig {
            magnifiers: sample_magnifiers(magnifiers),
            selected: Some(1),
            onboarding_done: true, // the guide is a separate window, tested below
            ..AppConfig::default()
        }
    }

    /// Renders the real panel headlessly and returns how far its content sticks out
    /// past the window's right edge (0 = everything fits).
    fn overflow(
        lang: Lang,
        width: f32,
        ui_scale: f32,
        tab: Tab,
        high_contrast: bool,
        magnifiers: usize,
        view: HomeView,
    ) -> f32 {
        let ctx = egui::Context::default();
        let mut config = test_config(magnifiers);
        config.home_view = view;
        config.appearance.ui_scale = ui_scale;
        config.appearance.high_contrast = high_contrast;
        let mut app = App::with_config(&ctx, config, lang, None, None);
        app.tab = tab;
        if tab == Tab::Appearance {
            // The largest preview, with the "current" note and enabled buttons.
            app.pending_ui_scale = Some(*UI_SCALE_RANGE.end());
        }
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
                        // The Home list with no, one, a few and the most magnifiers.
                        let counts: &[usize] = if tab == Tab::Home {
                            &[0, 1, 3, 8]
                        } else {
                            &[1]
                        };
                        let views: &[HomeView] = if tab == Tab::Home {
                            &[HomeView::Cards, HomeView::List]
                        } else {
                            &[HomeView::Cards]
                        };
                        for &count in counts {
                            for &view in views {
                                for high_contrast in [false, true] {
                                    let over = overflow(
                                        lang,
                                        width,
                                        scale,
                                        tab,
                                        high_contrast,
                                        count,
                                        view,
                                    );
                                    assert!(
                                        over <= 1.0,
                                        "{lang:?} {tab:?} {view:?} ({count} magnifiers) at {width} px, {:.0}%, high contrast {high_contrast}: content is {over:.0} px too wide",
                                        scale * 100.0
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// The first-run guide (every step) and the tray notice stay inside the window.
    #[test]
    fn dialogs_fit_narrow_and_wide_windows() {
        for lang in Lang::ALL {
            for width in [320.0, 900.0] {
                for scale in [1.0, 2.0] {
                    for dialog in [
                        "onboarding_0",
                        "onboarding_1",
                        "onboarding_2",
                        "tray_notice",
                    ] {
                        let ctx = egui::Context::default();
                        let mut config = AppConfig::default();
                        config.appearance.ui_scale = scale;
                        let mut app = App::with_config(&ctx, config, lang, None, None);
                        let id = if dialog == "tray_notice" {
                            app.show_onboarding = false;
                            app.show_tray_notice = true;
                            Id::unique("tray_notice")
                        } else {
                            app.onboarding_step = dialog[11..].parse().unwrap();
                            Id::unique("onboarding")
                        };
                        let input = || RawInput {
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 720.0))),
                            ..Default::default()
                        };
                        for _ in 0..4 {
                            let mut output = ctx.run_ui(input(), |ui| app.panel_ui(ui));
                            output.textures_delta.clear();
                        }
                        let rect = ctx
                            .memory(|m| m.area_rect(id))
                            .unwrap_or_else(|| panic!("{dialog} not shown"));
                        assert!(
                            rect.left() >= -0.5 && rect.right() <= width + 0.5,
                            "{lang:?} {dialog} at {width} px, {:.0}%: {rect:?} outside the window",
                            scale * 100.0
                        );
                    }
                }
            }
        }
    }

    /// Renders the real panel with the GPU and saves PNG screenshots for visual review.
    /// Run with `cargo wtest render_screenshots -- --ignored` (output: `target/screenshots`).
    #[test]
    #[ignore = "needs a GPU; run explicitly to review the look"]
    fn render_screenshots() {
        use area_selector::config::ThemeMode;
        let dir = std::path::Path::new("target").join("screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let cases = [
            ("home_light", ThemeMode::Light, false, Tab::Home, 900.0, 1.0),
            ("home_dark", ThemeMode::Dark, false, Tab::Home, 900.0, 1.0),
            (
                "home_list_light",
                ThemeMode::Light,
                false,
                Tab::Home,
                900.0,
                1.0,
            ),
            (
                "home_list_hc_dark",
                ThemeMode::Dark,
                true,
                Tab::Home,
                900.0,
                1.0,
            ),
            (
                "home_list_narrow_dark",
                ThemeMode::Dark,
                false,
                Tab::Home,
                480.0,
                1.0,
            ),
            (
                "home_hc_light",
                ThemeMode::Light,
                true,
                Tab::Home,
                900.0,
                1.0,
            ),
            ("home_hc_dark", ThemeMode::Dark, true, Tab::Home, 900.0, 1.0),
            (
                "home_narrow_dark",
                ThemeMode::Dark,
                false,
                Tab::Home,
                480.0,
                1.0,
            ),
            (
                "hotkeys_dark",
                ThemeMode::Dark,
                false,
                Tab::Hotkeys,
                560.0,
                1.0,
            ),
            (
                "hotkeys_hc_light",
                ThemeMode::Light,
                true,
                Tab::Hotkeys,
                560.0,
                1.0,
            ),
            (
                "appearance_light",
                ThemeMode::Light,
                false,
                Tab::Appearance,
                560.0,
                1.0,
            ),
            (
                "appearance_hc_dark",
                ThemeMode::Dark,
                true,
                Tab::Appearance,
                560.0,
                1.0,
            ),
            (
                "home_200_dark",
                ThemeMode::Dark,
                false,
                Tab::Home,
                560.0,
                2.0,
            ),
        ];
        for (name, theme, high_contrast, tab, width, scale) in cases {
            let mut config = test_config(3);
            config.appearance.theme = theme;
            config.appearance.high_contrast = high_contrast;
            config.appearance.ui_scale = scale;
            if name.contains("list") {
                config.home_view = HomeView::List;
            }
            let mut app = App::with_config(&egui::Context::default(), config, Lang::Pt, None, None);
            app.tab = tab;
            app.applied_appearance = None; // apply to the harness context below
            let mut harness = egui_kittest::Harness::builder()
                .with_size(vec2(width, 820.0))
                .wgpu()
                .build_ui_state(
                    |ui, app: &mut App| {
                        app.apply_theme(ui.ctx());
                        app.panel_ui(ui);
                    },
                    app,
                );
            harness.run();
            let image = harness.render().expect("render");
            image.save(dir.join(format!("{name}.png"))).unwrap();
        }

        // A magnifier's settings window.
        let windows = [
            ("settings_light", ThemeMode::Light, false, 480.0, 1.0),
            ("settings_hc_dark", ThemeMode::Dark, true, 480.0, 1.0),
            ("settings_narrow_dark", ThemeMode::Dark, false, 340.0, 1.5),
            ("settings_delete_light", ThemeMode::Light, false, 480.0, 1.0),
            ("settings_delete_hc_dark", ThemeMode::Dark, true, 340.0, 1.5),
        ];
        for (name, theme, high_contrast, width, scale) in windows {
            let appearance = Appearance {
                theme,
                high_contrast,
                ui_scale: scale,
                ..Appearance::default()
            };
            let mut window = WindowState::new(i18n::tr(Lang::Pt));
            window.confirm_delete = name.contains("delete");
            let state = (sample_magnifiers(1).remove(0), window);
            let mut harness = egui_kittest::Harness::builder()
                .with_size(vec2(width, 1100.0))
                .wgpu()
                .build_ui_state(
                    move |ui, (settings, window): &mut (MagnifierSettings, WindowState)| {
                        theme::apply(ui.ctx(), &appearance);
                        magnifier_window::window_ui(ui, settings, window);
                    },
                    state,
                );
            harness.run();
            let image = harness.render().expect("render");
            image.save(dir.join(format!("{name}.png"))).unwrap();
        }
    }

    /// The settings window fits every width, text size, language and colour scheme.
    #[test]
    fn settings_window_fits() {
        for lang in Lang::ALL {
            for width in [320.0, 480.0, 800.0] {
                for scale in [1.0, 2.0] {
                    for high_contrast in [false, true] {
                        for confirm_delete in [false, true] {
                            let ctx = egui::Context::default();
                            let appearance = Appearance {
                                high_contrast,
                                ui_scale: scale,
                                ..Appearance::default()
                            };
                            theme::apply(&ctx, &appearance);
                            let mut settings = sample_magnifiers(3).remove(2);
                            settings.name = "Chat do jogo com um nome bem comprido".to_owned();
                            let mut state = WindowState::new(i18n::tr(lang));
                            state.confirm_delete = confirm_delete;
                            let input = || RawInput {
                                screen_rect: Some(Rect::from_min_size(
                                    Pos2::ZERO,
                                    vec2(width, 720.0),
                                )),
                                ..Default::default()
                            };
                            for _ in 0..4 {
                                state.overflow = 0.0;
                                let mut output = ctx.run_ui(input(), |ui| {
                                    magnifier_window::window_ui(ui, &mut settings, &mut state);
                                });
                                output.textures_delta.clear();
                            }
                            assert!(
                                state.overflow <= 1.0,
                                "{lang:?} at {width} px, {:.0}%, hc {high_contrast}, confirm {confirm_delete}: {:.0} px too wide",
                                scale * 100.0,
                                state.overflow
                            );
                            if confirm_delete {
                                let id = magnifier_window::delete_dialog_id(settings.id);
                                let rect = ctx
                                    .memory(|m| m.area_rect(id))
                                    .expect("delete dialog not shown");
                                assert!(
                                    rect.left() >= -0.5 && rect.right() <= width + 0.5,
                                    "{lang:?} delete dialog at {width} px, {:.0}%: {rect:?} outside the window",
                                    scale * 100.0
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Simulates a finished selection for the current selector target.
    fn finish_selection(app: &mut App, ctx: &egui::Context, target: SelectorTarget) {
        app.selector_target = target;
        app.selector = Some(Arc::new(Mutex::new(SelectorState {
            selector: AreaSelector::capture_screen(),
            outcome: Some(SelectorOutcome::Selected(area())),
        })));
        app.reap_selector(ctx);
    }

    fn open_count(app: &App) -> usize {
        app.magnifiers.iter().filter(|e| e.lupa.is_some()).count()
    }

    #[test]
    fn adding_creates_selects_and_opens_a_magnifier() {
        let ctx = egui::Context::default();
        let mut app = App::with_config(&ctx, test_config(1), Lang::Pt, None, None);
        finish_selection(&mut app, &ctx, SelectorTarget::New);
        assert_eq!(app.magnifiers.len(), 2);
        let new = lock(&app.magnifiers[1].settings).clone();
        assert_eq!(new.id, 2);
        assert_eq!(new.name, "Lupa 2");
        assert_eq!(new.area, Some(area()));
        assert!(
            new.open && app.magnifiers[1].lupa.is_some(),
            "opens right away"
        );
        assert_eq!(app.config.selected, Some(2), "the new one is selected");
    }

    #[test]
    fn cannot_add_more_than_the_limit() {
        let ctx = egui::Context::default();
        let mut app = App::with_config(
            &ctx,
            test_config(config::MAX_MAGNIFIERS),
            Lang::Pt,
            None,
            None,
        );
        assert!(!app.can_add_magnifier());
        finish_selection(&mut app, &ctx, SelectorTarget::New);
        assert_eq!(app.magnifiers.len(), config::MAX_MAGNIFIERS);
    }

    #[test]
    fn selecting_an_area_updates_that_magnifier_only() {
        let ctx = egui::Context::default();
        let mut cfg = test_config(3);
        cfg.magnifiers[1].area = None;
        let mut app = App::with_config(&ctx, cfg, Lang::Pt, None, None);
        finish_selection(&mut app, &ctx, SelectorTarget::Magnifier(2));
        assert_eq!(lock(&app.magnifiers[1].settings).area, Some(area()));
        assert_eq!(app.magnifiers.len(), 3, "no new magnifier");
    }

    #[test]
    fn open_magnifiers_reopen_on_start() {
        let ctx = egui::Context::default();
        let mut cfg = test_config(3);
        cfg.magnifiers[0].open = true;
        cfg.magnifiers[2].open = true; // has no area: stays closed
        let app = App::with_config(&ctx, cfg, Lang::Pt, None, None);
        let open: Vec<bool> = app.magnifiers.iter().map(|e| e.lupa.is_some()).collect();
        assert_eq!(open, [true, false, false]);
    }

    #[test]
    fn hotkeys_act_on_all_or_on_the_selected_magnifier() {
        let ctx = egui::Context::default();
        let mut app = App::with_config(&ctx, test_config(3), Lang::Pt, None, None);
        let passthrough = |app: &App| -> Vec<bool> {
            app.magnifiers
                .iter()
                .map(|e| lock(&e.settings).passthrough)
                .collect()
        };

        // Passthrough: mixed → all on → all off.
        app.run_action(&ctx, Action::TogglePassthrough);
        assert_eq!(passthrough(&app), [true, true, true]);
        app.run_action(&ctx, Action::TogglePassthrough);
        assert_eq!(passthrough(&app), [false, false, false]);

        // Show/hide: none open → open all that have an area → close all.
        app.run_action(&ctx, Action::ToggleLupa);
        assert_eq!(open_count(&app), 2);
        app.run_action(&ctx, Action::ToggleLupa);
        assert_eq!(open_count(&app), 0);

        // Zoom acts on the selected magnifier; the selection cycles.
        app.run_action(&ctx, Action::CycleSelection);
        assert_eq!(app.config.selected, Some(2));
        app.run_action(&ctx, Action::ZoomIn);
        let zooms: Vec<f32> = app
            .magnifiers
            .iter()
            .map(|e| lock(&e.settings).zoom)
            .collect();
        assert_eq!(zooms, [2.0, 2.25, 2.0]);
    }

    #[test]
    fn deleting_fixes_the_selection_and_the_saved_list() {
        let ctx = egui::Context::default();
        let mut cfg = test_config(3);
        cfg.selected = Some(2);
        let mut app = App::with_config(&ctx, cfg, Lang::Pt, None, None);
        app.delete_magnifier(2);
        app.sync_config();
        let ids: Vec<u32> = app.config.magnifiers.iter().map(|m| m.id).collect();
        assert_eq!(ids, [1, 3]);
        assert_eq!(app.config.selected, Some(1));
    }

    /// Side-by-side cards share the taller card's height, without extra space (egui's
    /// `set_min_height` counts from the cursor, which once doubled the row height).
    #[test]
    fn side_by_side_cards_have_equal_height() {
        for lang in Lang::ALL {
            for scale in [1.0, 1.5] {
                let ctx = egui::Context::default();
                let mut config = test_config(3);
                config.appearance.ui_scale = scale;
                let mut app = App::with_config(&ctx, config, lang, None, None);
                let input = || RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
                    ..Default::default()
                };
                for _ in 0..4 {
                    let mut output = ctx.run_ui(input(), |ui| app.panel_ui(ui));
                    output.textures_delta.clear();
                }
                let (block, card) = app.card_row;
                assert!(
                    card > 50.0,
                    "{lang:?} {scale}: cards should be side by side"
                );
                assert!(
                    (block - card).abs() < 2.0,
                    "{lang:?} {scale}: row takes {block:.1} px but cards are {card:.1} px"
                );
            }
        }
    }

    /// Text-size harness on the Appearance tab (English labels).
    fn text_size_harness(height: f32) -> egui_kittest::Harness<'static, App> {
        let mut app = App::with_config(
            &egui::Context::default(),
            test_config(1),
            Lang::En,
            None,
            None,
        );
        app.tab = Tab::Appearance;
        app.applied_appearance = None; // apply to the harness context below
        let mut harness = egui_kittest::Harness::builder()
            .with_size(vec2(600.0, height))
            .build_ui_state(
                |ui, app: &mut App| {
                    app.apply_theme(ui.ctx());
                    app.panel_ui(ui);
                },
                app,
            );
        harness.run_steps(4);
        harness
    }

    fn body_font(harness: &egui_kittest::Harness<'_, App>) -> f32 {
        TextStyle::Body
            .resolve(&harness.ctx.style_of(harness.ctx.theme()))
            .size
    }

    /// − / + only change the preview; the panel resizes on Apply, and Reset drops the
    /// previewed size.
    #[test]
    fn text_size_changes_only_on_apply() {
        let mut harness = text_size_harness(820.0);
        let normal = body_font(&harness);
        for _ in 0..2 {
            harness.get_by_label("Increase text size").click();
            harness.run_steps(2);
        }
        assert_eq!(harness.state().pending_ui_scale, Some(1.2));
        assert_eq!(harness.state().config.appearance.ui_scale, 1.0);
        assert_eq!(body_font(&harness), normal, "the panel must not resize yet");

        harness.get_by_label("Apply").click();
        harness.run_steps(3);
        assert_eq!(harness.state().config.appearance.ui_scale, 1.2);
        assert!((body_font(&harness) - normal * 1.2).abs() < 0.01);

        harness.get_by_label("Increase text size").click();
        harness.run_steps(2);
        harness.get_by_label("Reset").click();
        harness.run_steps(2);
        assert_eq!(harness.state().pending_ui_scale, Some(1.2));
        assert_eq!(harness.state().config.appearance.ui_scale, 1.2);
    }

    /// After Apply the panel scrolls so the Apply button is back where it was (under
    /// the mouse), even though the larger text pushes it far down.
    #[test]
    fn apply_keeps_text_size_control_in_place() {
        let mut harness = text_size_harness(820.0);
        while harness.state().pending_ui_scale != Some(*UI_SCALE_RANGE.end()) {
            harness.get_by_label("Increase text size").click();
            harness.run_steps(2);
        }
        let before = harness.get_by_label("Apply").rect().top();
        harness.get_by_label("Apply").click();
        harness.run_steps(8);
        assert_eq!(harness.state().config.appearance.ui_scale, 2.0);
        let after = harness.get_by_label("Apply").rect().top();
        assert!(
            (after - before).abs() <= 1.0,
            "Apply moved from {before:.0} to {after:.0}"
        );
        assert!(harness.state().scale_anchor.is_none());
    }
}
