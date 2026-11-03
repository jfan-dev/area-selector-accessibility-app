use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub start: SerializablePos,
    pub end: SerializablePos,
}

impl Area {
    /// Converts an area measured in egui points to physical pixels, the unit that
    /// screen capture uses. Needed on displays with Windows scaling above 100%.
    pub fn to_physical(&self, pixels_per_point: f32) -> Self {
        let scale = |p: SerializablePos| SerializablePos {
            x: p.x * pixels_per_point,
            y: p.y * pixels_per_point,
        };
        Self {
            start: scale(self.start),
            end: scale(self.end),
        }
    }

    /// Normalized `(x, y, width, height)` in whole pixels, regardless of drag direction.
    pub fn to_rect_px(&self) -> (u32, u32, u32, u32) {
        let x = self.start.x.min(self.end.x).round() as u32;
        let y = self.start.y.min(self.end.y).round() as u32;
        let w = (self.start.x - self.end.x).abs().round() as u32;
        let h = (self.start.y - self.end.y).abs().round() as u32;
        (x, y, w, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(sx: f32, sy: f32, ex: f32, ey: f32) -> Area {
        Area {
            start: SerializablePos { x: sx, y: sy },
            end: SerializablePos { x: ex, y: ey },
        }
    }

    #[test]
    fn rect_from_top_left_drag() {
        assert_eq!(
            area(10.0, 20.0, 110.0, 70.0).to_rect_px(),
            (10, 20, 100, 50)
        );
    }

    #[test]
    fn rect_is_normalized_for_reverse_drag() {
        assert_eq!(
            area(110.0, 70.0, 10.0, 20.0).to_rect_px(),
            (10, 20, 100, 50)
        );
        assert_eq!(
            area(110.0, 20.0, 10.0, 70.0).to_rect_px(),
            (10, 20, 100, 50)
        );
    }

    #[test]
    fn rect_rounds_fractional_coordinates() {
        assert_eq!(area(10.4, 20.6, 50.5, 40.2).to_rect_px(), (10, 21, 40, 20));
    }

    #[test]
    fn zero_size_area() {
        assert_eq!(area(5.0, 5.0, 5.0, 5.0).to_rect_px(), (5, 5, 0, 0));
    }

    #[test]
    fn to_physical_scales_both_corners() {
        let a = area(10.0, 20.0, 110.0, 70.0).to_physical(1.5);
        assert_eq!(a, area(15.0, 30.0, 165.0, 105.0));
        assert_eq!(a.to_rect_px(), (15, 30, 150, 75));
    }

    #[test]
    fn to_physical_at_100_percent_is_identity() {
        let a = area(10.0, 20.0, 110.0, 70.0);
        assert_eq!(a.to_physical(1.0), a);
    }

    #[test]
    fn area_serde_round_trip() {
        let a = area(1.5, 2.5, 3.5, 4.5);
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Area>(&json).unwrap(), a);
    }
}
