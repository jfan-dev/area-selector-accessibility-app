use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Frame};
use crate::capture;
use std::time::Duration;

pub struct LupaApp {
    area: (u32, u32, u32, u32),
    zoom: f32,
    texture: Option<TextureHandle>,
    buffer: Vec<u8>,
    interactive: bool,
}

impl LupaApp {
    pub fn new(area: (u32, u32, u32, u32), zoom: f32, interactive: bool) -> Self {
        Self {
            area,
            zoom,
            texture: None,
            buffer: Vec::new(),
            interactive,
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(img) = capture::capture_area(self.area.0, self.area.1, self.area.2, self.area.3) {
            let size = [img.width() as usize, img.height() as usize];
            self.buffer.clear();
            self.buffer.reserve(size[0] * size[1] * 4);
            for p in img.pixels() {
                self.buffer.extend_from_slice(&p.0);
            }

            let image = ColorImage::from_rgba_unmultiplied(size, &self.buffer);

            if let Some(tex) = &mut self.texture {
                tex.set(image, TextureOptions::LINEAR);
            } else {
                self.texture = Some(ctx.load_texture(
                    "lupa_texture",
                    image,
                    TextureOptions::LINEAR,
                ));
            }
        }
    }
}

impl eframe::App for LupaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_texture(ctx);

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);

                if let Some(tex) = &self.texture {
                    let size = tex.size_vec2() * self.zoom;
                    ui.image((tex.id(), size));
                } else {
                    ui.label("⌛ Carregando...");
                }

                let color = if self.interactive {
                    egui::Color32::from_rgb(0, 255, 0)
                } else {
                    egui::Color32::from_rgb(180, 180, 180)
                };
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(3.0, color));
            });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
