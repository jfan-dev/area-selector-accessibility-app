use crate::{capture, icon};
use eframe::egui::{self, ColorImage, Frame, TextureHandle, TextureOptions, ViewportBuilder};
use std::time::Duration;

/// Screen region magnified by the lupa: `(x, y, width, height)` in pixels.
pub type CaptureRect = (u32, u32, u32, u32);

/// Result of one frame of the magnifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LupaOutcome {
    Open,
    /// The user pressed ESC (interactive mode) or the window was closed.
    Closed,
}

/// Magnifier window state. All fields that affect the window (zoom, position,
/// passthrough) can be changed while it is open; the next frame applies them.
pub struct LupaApp {
    pub area: CaptureRect,
    pub zoom: f32,
    pub position: [f32; 2],
    /// When true, mouse clicks go through the window to whatever is underneath.
    pub passthrough: bool,
    texture: Option<TextureHandle>,
    buffer: Vec<u8>,
}

impl LupaApp {
    pub const TITLE: &'static str = "Lupa";

    pub fn new(area: CaptureRect, zoom: f32, position: [f32; 2], passthrough: bool) -> Self {
        Self {
            area,
            zoom,
            position,
            passthrough,
            texture: None,
            buffer: Vec::new(),
        }
    }

    /// Borderless, transparent, always-on-top window sized to `area × zoom`.
    /// Re-sent every frame: egui only issues window commands for fields that changed.
    pub fn viewport_builder(&self) -> ViewportBuilder {
        let (_, _, w, h) = self.area;
        let builder = match icon::app_icon() {
            Some(icon) => ViewportBuilder::default().with_icon(icon),
            None => ViewportBuilder::default(),
        };
        builder
            .with_title(Self::TITLE)
            .with_inner_size([w as f32 * self.zoom, h as f32 * self.zoom])
            .with_position(self.position)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(self.passthrough)
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let (x, y, w, h) = self.area;
        if let Some(img) = capture::capture_area(x, y, w, h) {
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
                self.texture =
                    Some(ctx.load_texture("lupa_texture", image, TextureOptions::LINEAR));
            }
        }
    }

    /// Captures the area, draws it zoomed, and reports whether the window should close.
    pub fn ui(&mut self, ctx: &egui::Context) -> LupaOutcome {
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

                let color = if self.passthrough {
                    egui::Color32::from_rgb(180, 180, 180)
                } else {
                    egui::Color32::from_rgb(0, 255, 0)
                };
                painter.rect_stroke(
                    rect,
                    egui::Rounding::ZERO,
                    egui::Stroke::new(3.0_f32, color),
                );
            });

        ctx.request_repaint_after(Duration::from_millis(50));

        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            LupaOutcome::Closed
        } else {
            LupaOutcome::Open
        }
    }
}
