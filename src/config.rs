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

/// Look of the panel and the magnifier border.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Appearance {
    pub theme: ThemeMode,
    /// Extra-strong contrast (pure black/white, yellow/cyan accents).
    pub high_contrast: bool,
    /// Text and spacing scale of the panel, 1.0 = 100 %.
    pub ui_scale: f32,
    pub border_preset: BorderPreset,
    /// Magnifier border thickness in points.
    pub border_width: f32,
    /// Show the interactive/passthrough badge in the magnifier corner.
    pub show_mode_badge: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            high_contrast: false,
            ui_scale: 1.0,
            border_preset: BorderPreset::GreenGrey,
            border_width: 3.0,
            show_mode_badge: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppConfig {
    /// Selected screen region, in physical pixels.
    pub area: Option<Area>,
    pub lupa_pos_x: f32,
    pub lupa_pos_y: f32,
    pub lupa_zoom: f32,
    /// Magnifier refresh rate. Missing in configs from older versions → default.
    #[serde(default = "default_lupa_fps")]
    pub lupa_fps: u32,
    /// Whether the magnifier lets clicks through. Missing in older configs → off.
    #[serde(default)]
    pub lupa_passthrough: bool,
    /// Panel language; `None` follows the Windows display language.
    #[serde(default)]
    pub language: Option<Lang>,
    #[serde(default)]
    pub appearance: Appearance,
    /// System-wide shortcuts. Missing actions get their defaults on load.
    #[serde(default = "hotkeys::default_bindings")]
    pub hotkeys: Bindings,
}

fn default_lupa_fps() -> u32 {
    DEFAULT_LUPA_FPS
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            area: None,
            lupa_pos_x: 100.0,
            lupa_pos_y: 100.0,
            lupa_zoom: 1.0,
            lupa_fps: DEFAULT_LUPA_FPS,
            lupa_passthrough: false,
            language: None,
            appearance: Appearance::default(),
            hotkeys: hotkeys::default_bindings(),
        }
    }
}

impl AppConfig {
    /// Frame interval for the magnifier, with the fps clamped to a sane range.
    pub fn lupa_frame_interval(&self) -> Duration {
        let fps = self
            .lupa_fps
            .clamp(*LUPA_FPS_RANGE.start(), *LUPA_FPS_RANGE.end());
        Duration::from_secs(1) / fps
    }
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

/// Saves only the area, keeping the other settings already on disk.
pub fn save_area(area: &Area) -> io::Result<()> {
    let mut cfg = load_config();
    cfg.area = Some(*area);
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

    #[test]
    fn round_trip_preserves_values() {
        let path = temp_path("round_trip");
        let cfg = AppConfig {
            area: Some(Area {
                start: SerializablePos { x: 1.0, y: 2.0 },
                end: SerializablePos { x: 300.0, y: 400.0 },
            }),
            lupa_pos_x: 50.0,
            lupa_pos_y: 60.0,
            lupa_zoom: 2.5,
            lupa_fps: 45,
            lupa_passthrough: true,
            language: Some(Lang::En),
            appearance: Appearance {
                theme: ThemeMode::Dark,
                high_contrast: true,
                ui_scale: 1.5,
                border_preset: BorderPreset::BlueOrange,
                border_width: 6.0,
                show_mode_badge: false,
            },
            hotkeys: {
                let mut bindings = hotkeys::default_bindings();
                bindings.insert(hotkeys::Action::ZoomIn, String::new());
                bindings
            },
        };
        save_config_to(&path, &cfg).unwrap();
        assert_eq!(load_config_from(&path), cfg);
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

    #[test]
    fn reads_existing_config_format() {
        let json = r#"{
            "area": { "start": { "x": 10.0, "y": 20.0 }, "end": { "x": 110.0, "y": 70.0 } },
            "lupa_pos_x": 100.0,
            "lupa_pos_y": 100.0,
            "lupa_zoom": 1.0
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.area.unwrap().to_rect_px(), (10, 20, 100, 50));
        assert_eq!(
            cfg.lupa_fps, DEFAULT_LUPA_FPS,
            "older configs get the default fps"
        );
        assert!(!cfg.lupa_passthrough, "older configs start interactive");
        assert_eq!(
            cfg.language, None,
            "older configs follow the Windows language"
        );
        assert_eq!(cfg.appearance, Appearance::default());
        assert_eq!(cfg.hotkeys, hotkeys::default_bindings());
    }

    #[test]
    fn frame_interval_clamps_fps() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.lupa_frame_interval().as_millis(), 33);
        cfg.lupa_fps = 0;
        assert_eq!(cfg.lupa_frame_interval().as_millis(), 200);
        cfg.lupa_fps = 1000;
        assert_eq!(cfg.lupa_frame_interval().as_millis(), 16);
    }

    fn changed(zoom: f32) -> AppConfig {
        AppConfig {
            lupa_zoom: zoom,
            ..AppConfig::default()
        }
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
    fn partial_appearance_fills_missing_fields() {
        let json = r#"{ "lupa_pos_x": 1.0, "lupa_pos_y": 2.0, "lupa_zoom": 1.0,
                        "appearance": { "high_contrast": true } }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.appearance.high_contrast);
        assert_eq!(cfg.appearance.ui_scale, 1.0);
        assert_eq!(cfg.appearance.border_preset, BorderPreset::GreenGrey);
    }
}
