use crate::model::Area;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
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

pub fn config_path() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join("area_selector").join("config.json")
}

pub fn save_config(cfg: &AppConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        fs::write(path, json).ok();
    }
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_area(area: &Area) {
    let mut cfg = load_config();
    cfg.area = Some(area.clone());
    save_config(&cfg);
}
