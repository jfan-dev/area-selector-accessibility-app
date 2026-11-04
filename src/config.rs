use crate::hotkeys::{self, Bindings};
use crate::i18n::Lang;
use crate::model::Area;
use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const DEFAULT_LUPA_FPS: u32 = 30;
pub const LUPA_FPS_RANGE: std::ops::RangeInclusive<u32> = 5..=60;
pub const LUPA_ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.5..=5.0;
pub const UI_SCALE_RANGE: std::ops::RangeInclusive<f32> = 1.0..=2.0;
pub const BORDER_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 1.0..=10.0;

/// Light/dark mode of the panel.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// Follow the Windows light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

/// Colours of the magnifier border: (interactive, passthrough).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderPreset {
    /// Green / grey (the original look).
    #[default]
    GreenGrey,
    /// Blue / orange from the Okabe–Ito palette, distinguishable with colour blindness.
    BlueOrange,
    /// Yellow / magenta, maximum visibility.
    YellowMagenta,
}

impl BorderPreset {
    pub const ALL: [Self; 3] = [Self::GreenGrey, Self::BlueOrange, Self::YellowMagenta];

    /// `(interactive, passthrough)` border colours.
    pub fn colors(self) -> (Color32, Color32) {
        match self {
            Self::GreenGrey => (
                Color32::from_rgb(0, 220, 90),
                Color32::from_rgb(170, 170, 170),
            ),
            Self::BlueOrange => (
                Color32::from_rgb(0, 114, 178),
                Color32::from_rgb(230, 159, 0),
            ),
            Self::YellowMagenta => (
                Color32::from_rgb(255, 215, 0),
                Color32::from_rgb(255, 0, 255),
            ),
        }
    }
}

/// Look of the panel (shared by all windows).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Appearance {
    pub theme: ThemeMode,
    /// Extra-strong contrast (pure black/white, yellow/cyan accents).
    pub high_contrast: bool,
    /// Text and spacing scale of the panel, 1.0 = 100 %.
    pub ui_scale: f32,
    /// Magnifier border settings from before multiple magnifiers (each magnifier now
    /// has its own); read only to migrate old configs.
    #[serde(rename = "border_preset", skip_serializing)]
    pub legacy_border_preset: Option<BorderPreset>,
    #[serde(rename = "border_width", skip_serializing)]
    pub legacy_border_width: Option<f32>,
    #[serde(rename = "show_mode_badge", skip_serializing)]
    pub legacy_show_mode_badge: Option<bool>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            high_contrast: false,
            ui_scale: 1.0,
            legacy_border_preset: None,
            legacy_border_width: None,
            legacy_show_mode_badge: None,
        }
    }
}

/// Most magnifiers at once (each one captures the screen at its own frame rate).
pub const MAX_MAGNIFIERS: usize = 8;
pub const DEFAULT_ZOOM: f32 = 2.0;

/// Everything about one magnifier: its area and its own settings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct MagnifierSettings {
    /// Stable identifier (window ids, selection, tray menu).
    pub id: u32,
    pub name: String,
    /// Screen region, in physical pixels.
    pub area: Option<Area>,
    pub zoom: f32,
    /// Window position in points.
    pub pos_x: f32,
    pub pos_y: f32,
    pub fps: u32,
    /// Clicks go through the magnifier.
    pub passthrough: bool,
    /// Was open when the app closed: reopens on the next start.
    pub open: bool,
    pub border_preset: BorderPreset,
    /// Border thickness in points.
    pub border_width: f32,
    /// Show the name/mode badge in the corner.
    pub show_badge: bool,
}

impl Default for MagnifierSettings {
    fn default() -> Self {
        Self {
            id: 1,
            name: String::new(),
            area: None,
            zoom: DEFAULT_ZOOM,
            pos_x: 100.0,
            pos_y: 100.0,
            fps: DEFAULT_LUPA_FPS,
            passthrough: false,
            open: false,
            border_preset: BorderPreset::GreenGrey,
            border_width: 3.0,
            show_badge: true,
        }
    }
}

impl MagnifierSettings {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            ..Self::default()
        }
    }

    /// Frame interval, with the fps clamped to a sane range.
    pub fn frame_interval(&self) -> Duration {
        let fps = self
            .fps
            .clamp(*LUPA_FPS_RANGE.start(), *LUPA_FPS_RANGE.end());
        Duration::from_secs(1) / fps
    }
}

/// Top-level fields of configs from before multiple magnifiers; read only to migrate.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct LegacyFields {
    pub area: Option<Area>,
    pub lupa_pos_x: Option<f32>,
    pub lupa_pos_y: Option<f32>,
    pub lupa_zoom: Option<f32>,
    pub lupa_fps: Option<u32>,
    pub lupa_passthrough: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppConfig {
    /// The magnifiers, in display order.
    #[serde(default)]
    pub magnifiers: Vec<MagnifierSettings>,
    /// Magnifier targeted by the zoom / select-area hotkeys.
    #[serde(default)]
    pub selected: Option<u32>,
    /// Panel language; `None` follows the Windows display language.
    #[serde(default)]
    pub language: Option<Lang>,
    #[serde(default)]
    pub appearance: Appearance,
    /// System-wide shortcuts. Missing actions get their defaults on load.
    #[serde(default = "hotkeys::default_bindings")]
    pub hotkeys: Bindings,
    /// Closing the panel hides it to the system tray instead of quitting.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    /// The one-time explanation of close-to-tray was confirmed with "don't show again".
    #[serde(default)]
    pub tray_notice_shown: bool,
    /// The first-run guide was finished or skipped.
    #[serde(default)]
    pub onboarding_done: bool,
    /// Single-magnifier fields of older versions (never written).
    #[serde(flatten, skip_serializing)]
    pub legacy: LegacyFields,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            magnifiers: Vec::new(),
            selected: None,
            language: None,
            appearance: Appearance::default(),
            hotkeys: hotkeys::default_bindings(),
            close_to_tray: true,
            tray_notice_shown: false,
            onboarding_done: false,
            legacy: LegacyFields::default(),
        }
    }
}

impl AppConfig {
    /// Converts a config from before multiple magnifiers into one magnifier named
    /// `name`, then [`Self::normalize`]s. Configs already in the new format (even with
    /// all magnifiers deleted) are left alone.
    pub fn migrate(&mut self, name: &str) {
        let legacy = std::mem::take(&mut self.legacy);
        let appearance = &mut self.appearance;
        let border_preset = appearance.legacy_border_preset.take();
        let border_width = appearance.legacy_border_width.take();
        let show_badge = appearance.legacy_show_mode_badge.take();
        if self.magnifiers.is_empty() && legacy.lupa_zoom.is_some() {
            let defaults = MagnifierSettings::default();
            self.magnifiers.push(MagnifierSettings {
                id: 1,
                name: name.to_owned(),
                area: legacy.area,
                zoom: legacy.lupa_zoom.unwrap_or(defaults.zoom),
                pos_x: legacy.lupa_pos_x.unwrap_or(defaults.pos_x),
                pos_y: legacy.lupa_pos_y.unwrap_or(defaults.pos_y),
                fps: legacy.lupa_fps.unwrap_or(defaults.fps),
                passthrough: legacy.lupa_passthrough.unwrap_or(false),
                open: false,
                border_preset: border_preset.unwrap_or(defaults.border_preset),
                border_width: border_width.unwrap_or(defaults.border_width),
                show_badge: show_badge.unwrap_or(defaults.show_badge),
            });
        }
        self.normalize();
    }

    /// Unique ids, at most [`MAX_MAGNIFIERS`], and a selection that exists.
    pub fn normalize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut next = self.next_magnifier_id();
        for magnifier in &mut self.magnifiers {
            if magnifier.id == 0 || !seen.insert(magnifier.id) {
                magnifier.id = next;
                seen.insert(next);
                next += 1;
            }
        }
        self.magnifiers.truncate(MAX_MAGNIFIERS);
        let selected_exists = self
            .selected
            .is_some_and(|id| self.magnifiers.iter().any(|m| m.id == id));
        if !selected_exists {
            self.selected = self.magnifiers.first().map(|m| m.id);
        }
    }

    pub fn next_magnifier_id(&self) -> u32 {
        self.magnifiers.iter().map(|m| m.id).max().unwrap_or(0) + 1
    }

    pub fn can_add_magnifier(&self) -> bool {
        self.magnifiers.len() < MAX_MAGNIFIERS
    }

    /// The selected magnifier (or the first one).
    pub fn selected_magnifier_mut(&mut self) -> Option<&mut MagnifierSettings> {
        let id = self.selected;
        let index = self.magnifiers.iter().position(|m| Some(m.id) == id).or(
            if self.magnifiers.is_empty() {
                None
            } else {
                Some(0)
            },
        )?;
        self.magnifiers.get_mut(index)
    }
}

/// "Passthrough for all" hotkey: if any magnifier is interactive, all become
/// passthrough; if all already are, all become interactive.
pub fn all_passthrough_target(passthrough: &[bool]) -> bool {
    !passthrough.iter().all(|&p| p)
}

/// The next magnifier for the "select next" hotkey (wraps around).
pub fn next_selection(ids: &[u32], current: Option<u32>) -> Option<u32> {
    let index = current.and_then(|id| ids.iter().position(|&i| i == id));
    match index {
        Some(i) => ids.get((i + 1) % ids.len()).copied(),
        None => ids.first().copied(),
    }
}

/// Starting position for the `index`-th new magnifier, cascading so new magnifiers
/// don't open exactly on top of each other.
pub fn cascade_position(index: usize) -> [f32; 2] {
    let step = (index % 8) as f32 * 40.0;
    [40.0 + step, 40.0 + step]
}

/// How long settings must stay unchanged before they are written to disk, so dragging
/// a slider doesn't write the file on every frame.
pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(500);

/// What [`AutoSave::poll`] asks the caller to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoSaveAction {
    /// Nothing changed since the last save.
    Idle,
    /// Settings changed recently; poll again after this long.
    Wait(Duration),
    /// Settings have been stable for [`AUTOSAVE_DELAY`]; save them now.
    SaveNow,
}

/// Debounced auto-save: tracks the last saved settings and when they last changed.
/// Time is passed in, so the logic is testable without real timers.
#[derive(Debug, Clone)]
pub struct AutoSave {
    saved: AppConfig,
    last_seen: AppConfig,
    changed_at: Option<Instant>,
}

impl AutoSave {
    pub fn new(saved: &AppConfig) -> Self {
        Self {
            saved: saved.clone(),
            last_seen: saved.clone(),
            changed_at: None,
        }
    }

    /// Call once per frame with the current settings.
    pub fn poll(&mut self, current: &AppConfig, now: Instant) -> AutoSaveAction {
        if *current == self.saved {
            self.last_seen = current.clone();
            self.changed_at = None;
            return AutoSaveAction::Idle;
        }
        if *current != self.last_seen || self.changed_at.is_none() {
            self.last_seen = current.clone();
            self.changed_at = Some(now);
        }
        let elapsed = now.saturating_duration_since(self.changed_at.unwrap_or(now));
        if elapsed >= AUTOSAVE_DELAY {
            AutoSaveAction::SaveNow
        } else {
            AutoSaveAction::Wait(AUTOSAVE_DELAY - elapsed)
        }
    }

    /// Records that `cfg` is what is on disk now.
    pub fn mark_saved(&mut self, cfg: &AppConfig) {
        self.saved = cfg.clone();
        self.last_seen = cfg.clone();
        self.changed_at = None;
    }

    pub fn has_unsaved_changes(&self, current: &AppConfig) -> bool {
        *current != self.saved
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("area_selector")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn save_config(cfg: &AppConfig) -> io::Result<()> {
    save_config_to(&config_path(), cfg)
}

pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

/// Saves `area` as the selected magnifier's area (creating a magnifier named `name`
/// if there is none), keeping the other settings already on disk.
pub fn save_area(area: &Area, name: &str) -> io::Result<()> {
    let mut cfg = load_config();
    if cfg.magnifiers.is_empty() {
        cfg.magnifiers
            .push(MagnifierSettings::new(1, format!("{name} 1")));
        cfg.normalize();
    }
    if let Some(magnifier) = cfg.selected_magnifier_mut() {
        magnifier.area = Some(*area);
    }
    save_config(&cfg)
}

/// Writes the config as pretty JSON, creating parent directories as needed.
pub fn save_config_to(path: &Path, cfg: &AppConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(io::Error::other)?;
    fs::write(path, json)
}

/// Reads the config, falling back to defaults if the file is missing or invalid.
pub fn load_config_from(path: &Path) -> AppConfig {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return AppConfig::default(),
        Err(err) => {
            log::warn!("could not read {}: {err}; using defaults", path.display());
            return AppConfig::default();
        }
    };
    let mut cfg: AppConfig = serde_json::from_str(&data).unwrap_or_else(|err| {
        log::warn!("invalid config {}: {err}; using defaults", path.display());
        AppConfig::default()
    });
    cfg.hotkeys = hotkeys::with_defaults(std::mem::take(&mut cfg.hotkeys));
    let lang = Lang::resolve(cfg.language);
    cfg.migrate(&format!(
        "{} 1",
        crate::i18n::tr(lang).magnifier_default_name
    ));
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SerializablePos;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "area_selector_test_{}_{}",
                std::process::id(),
                name
            ))
            .join("config.json")
    }

    fn magnifier(id: u32, name: &str) -> MagnifierSettings {
        MagnifierSettings::new(id, name.to_owned())
    }

    #[test]
    fn round_trip_preserves_values() {
        let path = temp_path("round_trip");
        let mut first = magnifier(1, "Minimapa");
        first.area = Some(Area {
            start: SerializablePos { x: 1.0, y: 2.0 },
            end: SerializablePos { x: 300.0, y: 400.0 },
        });
        first.zoom = 2.5;
        first.fps = 45;
        first.passthrough = true;
        first.open = true;
        first.border_preset = BorderPreset::BlueOrange;
        first.border_width = 6.0;
        first.show_badge = false;
        let cfg = AppConfig {
            magnifiers: vec![first, magnifier(4, "Vida")],
            selected: Some(4),
            language: Some(Lang::En),
            appearance: Appearance {
                theme: ThemeMode::Dark,
                high_contrast: true,
                ui_scale: 1.5,
                ..Appearance::default()
            },
            hotkeys: {
                let mut bindings = hotkeys::default_bindings();
                bindings.insert(hotkeys::Action::ZoomIn, String::new());
                bindings
            },
            close_to_tray: false,
            tray_notice_shown: true,
            onboarding_done: true,
            legacy: LegacyFields::default(),
        };
        save_config_to(&path, &cfg).unwrap();
        assert_eq!(load_config_from(&path), cfg);
        let json = fs::read_to_string(&path).unwrap();
        assert!(!json.contains("lupa_zoom"), "old fields are not written");
        fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(
            load_config_from(&temp_path("missing")),
            AppConfig::default()
        );
    }

    #[test]
    fn corrupt_json_gives_defaults() {
        let path = temp_path("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not valid json").unwrap();
        assert_eq!(load_config_from(&path), AppConfig::default());
        fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    /// A config from before multiple magnifiers becomes one magnifier with the same
    /// area, zoom, position, fps, passthrough and border.
    #[test]
    fn migrates_single_magnifier_config() {
        let path = temp_path("legacy");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let json = r#"{
            "area": { "start": { "x": 10.0, "y": 20.0 }, "end": { "x": 110.0, "y": 70.0 } },
            "lupa_pos_x": 300.0,
            "lupa_pos_y": 200.0,
            "lupa_zoom": 2.5,
            "lupa_fps": 45,
            "lupa_passthrough": true,
            "appearance": { "border_preset": "blue_orange", "border_width": 5.0, "high_contrast": true }
        }"#;
        fs::write(&path, json).unwrap();
        let cfg = load_config_from(&path);
        assert_eq!(cfg.magnifiers.len(), 1);
        let m = &cfg.magnifiers[0];
        assert_eq!(m.area.unwrap().to_rect_px(), (10, 20, 100, 50));
        assert_eq!((m.zoom, m.pos_x, m.pos_y, m.fps), (2.5, 300.0, 200.0, 45));
        assert!(m.passthrough);
        assert_eq!(
            (m.border_preset, m.border_width),
            (BorderPreset::BlueOrange, 5.0)
        );
        assert!(m.name.ends_with(" 1"), "named like \"Lupa 1\": {}", m.name);
        assert_eq!(cfg.selected, Some(m.id));
        assert!(cfg.appearance.high_contrast, "panel settings are kept");
        assert_eq!(
            cfg.legacy,
            LegacyFields::default(),
            "migrated fields are cleared"
        );
        // Saving and loading again keeps the migrated magnifier as it is.
        save_config_to(&path, &cfg).unwrap();
        assert_eq!(load_config_from(&path), cfg);
        fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn deleting_every_magnifier_is_kept() {
        let mut cfg: AppConfig = serde_json::from_str(r#"{ "magnifiers": [] }"#).unwrap();
        cfg.migrate("Lupa 1");
        assert!(cfg.magnifiers.is_empty());
        assert_eq!(cfg.selected, None);
    }

    #[test]
    fn normalize_fixes_ids_limit_and_selection() {
        let mut cfg = AppConfig {
            magnifiers: (0..12).map(|_| magnifier(3, "x")).collect(),
            selected: Some(99),
            ..AppConfig::default()
        };
        cfg.normalize();
        assert_eq!(cfg.magnifiers.len(), MAX_MAGNIFIERS);
        let mut ids: Vec<u32> = cfg.magnifiers.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), MAX_MAGNIFIERS, "ids are unique");
        assert_eq!(cfg.selected, Some(cfg.magnifiers[0].id), "selection exists");
        assert!(!cfg.can_add_magnifier());
    }

    #[test]
    fn new_ids_are_never_reused_while_in_use() {
        let cfg = AppConfig {
            magnifiers: vec![magnifier(1, "a"), magnifier(5, "b")],
            ..AppConfig::default()
        };
        assert_eq!(cfg.next_magnifier_id(), 6);
        assert_eq!(AppConfig::default().next_magnifier_id(), 1);
    }

    #[test]
    fn passthrough_for_all_rule() {
        assert!(
            all_passthrough_target(&[false, true]),
            "any interactive → all on"
        );
        assert!(all_passthrough_target(&[false]));
        assert!(!all_passthrough_target(&[true, true]), "all on → all off");
    }

    #[test]
    fn selection_cycles_and_wraps() {
        let ids = [2, 5, 9];
        assert_eq!(next_selection(&ids, Some(2)), Some(5));
        assert_eq!(next_selection(&ids, Some(9)), Some(2));
        assert_eq!(next_selection(&ids, None), Some(2));
        assert_eq!(next_selection(&ids, Some(42)), Some(2));
        assert_eq!(next_selection(&[], Some(1)), None);
    }

    #[test]
    fn new_magnifiers_cascade() {
        assert_eq!(cascade_position(0), [40.0, 40.0]);
        assert_eq!(cascade_position(2), [120.0, 120.0]);
    }

    #[test]
    fn frame_interval_clamps_fps() {
        let mut m = MagnifierSettings::default();
        assert_eq!(m.frame_interval().as_millis(), 33);
        m.fps = 0;
        assert_eq!(m.frame_interval().as_millis(), 200);
        m.fps = 1000;
        assert_eq!(m.frame_interval().as_millis(), 16);
    }

    fn changed(value: f32) -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.appearance.ui_scale = value;
        cfg
    }

    #[test]
    fn autosave_is_idle_without_changes() {
        let cfg = AppConfig::default();
        let mut auto = AutoSave::new(&cfg);
        assert_eq!(auto.poll(&cfg, Instant::now()), AutoSaveAction::Idle);
        assert!(!auto.has_unsaved_changes(&cfg));
    }

    #[test]
    fn autosave_waits_then_saves_after_delay() {
        let mut auto = AutoSave::new(&AppConfig::default());
        let t0 = Instant::now();
        let cfg = changed(2.0);
        assert_eq!(auto.poll(&cfg, t0), AutoSaveAction::Wait(AUTOSAVE_DELAY));
        let ms = |n| t0 + Duration::from_millis(n);
        assert_eq!(
            auto.poll(&cfg, ms(200)),
            AutoSaveAction::Wait(Duration::from_millis(300))
        );
        assert_eq!(auto.poll(&cfg, ms(500)), AutoSaveAction::SaveNow);
    }

    #[test]
    fn autosave_restarts_delay_while_still_changing() {
        let mut auto = AutoSave::new(&AppConfig::default());
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        auto.poll(&changed(1.5), t0);
        // Still dragging the slider at 400 ms: the delay starts over.
        assert_eq!(
            auto.poll(&changed(1.8), ms(400)),
            AutoSaveAction::Wait(AUTOSAVE_DELAY)
        );
        assert!(matches!(
            auto.poll(&changed(1.8), ms(800)),
            AutoSaveAction::Wait(_)
        ));
        assert_eq!(auto.poll(&changed(1.8), ms(900)), AutoSaveAction::SaveNow);
    }

    #[test]
    fn autosave_is_idle_after_mark_saved_or_revert() {
        let original = AppConfig::default();
        let mut auto = AutoSave::new(&original);
        let t0 = Instant::now();
        let cfg = changed(3.0);
        auto.poll(&cfg, t0);
        auto.mark_saved(&cfg);
        assert_eq!(auto.poll(&cfg, t0), AutoSaveAction::Idle);

        // Changing a value and putting it back before the delay needs no save.
        auto.poll(&changed(4.0), t0);
        assert_eq!(auto.poll(&cfg, t0), AutoSaveAction::Idle);
    }

    #[test]
    fn partial_settings_fill_missing_fields() {
        let json = r#"{ "appearance": { "high_contrast": true },
                        "magnifiers": [ { "id": 3, "name": "Chat" } ] }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.appearance.high_contrast);
        assert_eq!(cfg.appearance.ui_scale, 1.0);
        let m = &cfg.magnifiers[0];
        assert_eq!((m.id, m.zoom, m.fps), (3, DEFAULT_ZOOM, DEFAULT_LUPA_FPS));
        assert_eq!(m.border_preset, BorderPreset::GreenGrey);
    }
}
