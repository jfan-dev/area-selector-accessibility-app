use serde::{Deserialize, Serialize};
use eframe::egui;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct SerializablePos {
    pub x: f32,
    pub y: f32,
}

impl From<egui::Pos2> for SerializablePos {
    fn from(p: egui::Pos2) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<SerializablePos> for egui::Pos2 {
    fn from(p: SerializablePos) -> Self {
        egui::pos2(p.x, p.y)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Area {
    pub start: SerializablePos,
    pub end: SerializablePos,
}
