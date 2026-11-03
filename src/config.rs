use crate::model::Area;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub area: Option<Area>,
    pub lupa_pos_x: f32,
    pub lupa_pos_y: f32,
    pub lupa_zoom: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            area: None,
            lupa_pos_x: 100.0,
            lupa_pos_y: 100.0,
            lupa_zoom: 1.0,
        }
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

pub fn save_config(cfg: &AppConfig) {
    save_config_to(&config_path(), cfg);
}

pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

pub fn save_area(area: &Area) {
    let mut cfg = load_config();
    cfg.area = Some(*area);
    save_config(&cfg);
}

/// Writes the config as pretty JSON, creating parent directories as needed.
pub fn save_config_to(path: &Path, cfg: &AppConfig) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        fs::write(path, json).ok();
    }
}

/// Reads the config, falling back to defaults if the file is missing or invalid.
pub fn load_config_from(path: &Path) -> AppConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
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
        };
        save_config_to(&path, &cfg);
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
    }
}
