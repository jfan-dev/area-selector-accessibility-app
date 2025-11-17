use eframe::egui::{self, *};
use crate::{config, model::Area as ScreenArea};

pub struct AreaSelector {
    selecting: bool,
    start: Option<Pos2>,
    end: Option<Pos2>,
}

impl Default for AreaSelector {
    fn default() -> Self {
        Self {
            selecting: false,
            start: None,
            end: None,
        }
    }
}

impl eframe::App for AreaSelector {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);

                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 12),
                );

                if ctx.input(|i| i.pointer.primary_down()) {
                    if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                        if !self.selecting {
                            self.selecting = true;
                            self.start = Some(pos);
                        }
                        self.end = Some(pos);
                    }
                } else if self.selecting {
                    self.selecting = false;

                    if let (Some(start), Some(end)) = (self.start, self.end) {
                        let area = ScreenArea {
                            start: start.into(),
                            end: end.into(),
                        };
                        config::save_area(&area);
                        println!("✅ Área salva: {:?}", area);
                        std::process::exit(0);
                    }
                }

                if let (Some(start), Some(end)) = (self.start, self.end) {
                    painter.rect(
                        Rect::from_two_pos(start, end),
                        0.0,
                        Color32::from_rgba_unmultiplied(100, 150, 255, 100),
                        Stroke::new(1.0, Color32::BLUE),
                    );
                }

                ui.label("Clique e arraste para selecionar uma área. ESC cancela.");
            });

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            std::process::exit(0);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
}
