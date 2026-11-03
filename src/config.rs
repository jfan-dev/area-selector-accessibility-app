use crate::model::Area;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_LUPA_FPS: u32 = 30;
pub const LUPA_FPS_RANGE: std::ops::RangeInclusive<u32> = 5..=60;

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
        }
    }
}

impl AppConfig {
    /// Frame interval for the magnifier, with the fps clamped to a sane range.
    pub fn lupa_frame_interval(&self) -> std::time::Duration {
        let fps = self
            .lupa_fps
            .clamp(*LUPA_FPS_RANGE.start(), *LUPA_FPS_RANGE.end());
        std::time::Duration::from_secs(1) / fps
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
    serde_json::from_str(&data).unwrap_or_else(|err| {
        log::warn!("invalid config {}: {err}; using defaults", path.display());
        AppConfig::default()
    })
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
}
