use crate::{capture, i18n, icon, model::Area};
use eframe::egui::{self, *};

/// Result of one frame of the area selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectorOutcome {
    /// Still waiting for the user to finish a drag.
    Pending,
    /// The user released the mouse after dragging. Area is in physical pixels.
    Selected(Area),
    /// The user pressed ESC or closed the window.
    Cancelled,
}

/// Fullscreen drag-to-select overlay.
///
/// The background is a screenshot taken just before the overlay opens, drawn dimmed,
/// with the selected region shown at full brightness. This does not depend on window
/// transparency: secondary windows were not see-through with eframe's glow renderer
/// on Windows (found with eframe 0.26; not re-tested after the upgrade to 0.37).
#[derive(Default)]
pub struct AreaSelector {
    selecting: bool,
    start: Option<Pos2>,
    end: Option<Pos2>,
    screenshot: Option<ColorImage>,
    texture: Option<TextureHandle>,
    hint: &'static str,
}

impl AreaSelector {
    pub const TITLE: &'static str = "Selecionar Área";

    /// Takes a screenshot of the primary monitor to use as the overlay background.
    /// Must be called before the overlay window is shown, so it doesn't capture itself.
    pub fn capture_screen() -> Self {
        let size = primary_screen_size_px();
        let screenshot = capture::capture_area(0, 0, size.x as u32, size.y as u32);
        if screenshot.is_none() {
            log::warn!("selector screenshot failed; using a plain overlay");
        }
        Self {
            screenshot,
            hint: i18n::PT.selector_hint,
            ..Self::default()
        }
    }

    /// Sets the instructions shown at the top of the overlay (translated text).
    pub fn with_hint(mut self, hint: &'static str) -> Self {
        self.hint = hint;
        self
    }

    /// Borderless, always-on-top window covering the primary monitor
    /// (`screen_size` in points).
    pub fn viewport_builder(screen_size: Vec2) -> ViewportBuilder {
        let builder = match icon::app_icon() {
            Some(icon) => ViewportBuilder::default().with_icon(icon),
            None => ViewportBuilder::default(),
        };
        builder
            .with_title(Self::TITLE)
            .with_inner_size(screen_size)
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
    }

    fn background(&mut self, ctx: &egui::Context) -> Option<(TextureId, Vec2)> {
        if let Some(image) = self.screenshot.take() {
            self.texture =
                Some(ctx.load_texture("selector_screenshot", image, TextureOptions::LINEAR));
        }
        let tex = self.texture.as_ref()?;
        // One screenshot pixel per physical screen pixel.
        Some((tex.id(), tex.size_vec2() / ctx.pixels_per_point()))
    }

    /// Draws the overlay and reports whether a selection was made.
    pub fn ui(&mut self, ui: &mut Ui) -> SelectorOutcome {
        let ctx = &ui.ctx().clone();
        let mut outcome = SelectorOutcome::Pending;
        let background = self.background(ctx);

        CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter_at(rect);
            let full_uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

            match background {
                Some((tex, size)) => {
                    let screen = Rect::from_min_size(Pos2::ZERO, size);
                    painter.image(tex, screen, full_uv, Color32::WHITE);
                    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 110));
                }
                // Screenshot failed: plain dark overlay so the selector still works.
                None => {
                    painter.rect_filled(rect, 0.0, Color32::from_gray(30));
                }
            }

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
                    let area = Area {
                        start: start.into(),
                        end: end.into(),
                    };
                    let ppp = ctx.pixels_per_point();
                    outcome = SelectorOutcome::Selected(area.to_physical(ppp));
                }
            }

            if let (Some(start), Some(end)) = (self.start, self.end) {
                let selection = Rect::from_two_pos(start, end);
                if let Some((tex, size)) = background {
                    // Show the selected region undimmed.
                    let uv = Rect::from_min_max(
                        (selection.min.to_vec2() / size).to_pos2(),
                        (selection.max.to_vec2() / size).to_pos2(),
                    );
                    painter.image(tex, selection, uv, Color32::WHITE);
                }
                painter.rect(
                    selection,
                    CornerRadius::ZERO,
                    Color32::from_rgba_unmultiplied(100, 150, 255, 40),
                    Stroke::new(2.0_f32, Color32::from_rgb(80, 140, 255)),
                    StrokeKind::Middle,
                );
            }

            ui.label(
                RichText::new(self.hint)
                    .color(Color32::WHITE)
                    .background_color(Color32::from_black_alpha(160)),
            );
        });

        if ctx.input(|i| i.key_pressed(Key::Escape) || i.viewport().close_requested()) {
            outcome = SelectorOutcome::Cancelled;
        }

        // No timed repaint: egui repaints on every mouse/keyboard event, which is all
        // the selector needs (it used to redraw every 30 ms).
        outcome
    }
}

/// Primary monitor size in points, falling back to the Win32 screen metrics when
/// winit has not reported a monitor size yet.
pub fn screen_size(ctx: &egui::Context) -> Vec2 {
    if let Some(size) = ctx.input(|i| i.viewport().monitor_size) {
        if size.x > 1.0 && size.y > 1.0 {
            return size;
        }
    }
    primary_screen_size_px() / ctx.pixels_per_point()
}

/// Primary monitor size in physical pixels (Win32 `GetSystemMetrics`).
/// Used before any window exists, e.g. for the standalone `seletor` mode.
pub fn primary_screen_size_px() -> Vec2 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    vec2(w as f32, h as f32)
}
