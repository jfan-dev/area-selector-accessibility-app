use crate::i18n::{self, Strings};
use crate::{capture, icon, model::Area};
use eframe::egui::{self, *};
use std::time::{Duration, Instant};

/// Result of one frame of the area selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectorOutcome {
    /// Still waiting for the user to finish a drag.
    Pending,
    /// The user released the mouse after dragging. Area is in physical pixels and at
    /// least [`crate::model::MIN_AREA_SIZE`] on each side.
    Selected(Area),
    /// The user pressed ESC, right-clicked or closed the window.
    Cancelled,
}

/// How long the "area too small" message stays visible.
const TOO_SMALL_MESSAGE: Duration = Duration::from_millis(2500);

/// Fullscreen drag-to-select overlay.
///
/// The background is a screenshot taken just before the overlay opens, drawn dimmed,
/// with the selected region shown at full brightness. This does not depend on window
/// transparency: secondary windows were not see-through with eframe's glow renderer
/// on Windows (found with eframe 0.26; not re-tested after the upgrade to 0.37).
pub struct AreaSelector {
    selecting: bool,
    start: Option<Pos2>,
    end: Option<Pos2>,
    screenshot: Option<ColorImage>,
    texture: Option<TextureHandle>,
    strings: &'static Strings,
    /// Set when a selection was rejected for being too small.
    too_small_until: Option<Instant>,
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
            selecting: false,
            start: None,
            end: None,
            screenshot,
            texture: None,
            strings: &i18n::PT,
            too_small_until: None,
        }
    }

    /// Sets the language of the instructions and messages.
    pub fn with_strings(mut self, strings: &'static Strings) -> Self {
        self.strings = strings;
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
        let ppp = ctx.pixels_per_point();
        ctx.set_cursor_icon(CursorIcon::Crosshair);

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
                        self.too_small_until = None;
                    }
                    self.end = Some(pos);
                }
            } else if self.selecting {
                self.selecting = false;
                if let (Some(start), Some(end)) = (self.start, self.end) {
                    let area = Area {
                        start: start.into(),
                        end: end.into(),
                    }
                    .to_physical(ppp);
                    if area.is_usable() {
                        outcome = SelectorOutcome::Selected(area);
                    } else {
                        // A click or a tiny drag: keep the overlay open and explain.
                        self.start = None;
                        self.end = None;
                        self.too_small_until = Some(Instant::now() + TOO_SMALL_MESSAGE);
                    }
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
                // Live size in physical pixels, next to the cursor.
                let w = (selection.width() * ppp).round() as u32;
                let h = (selection.height() * ppp).round() as u32;
                let text = (self.strings.selector_size)(w, h);
                draw_label(&painter, ui, end + vec2(16.0, 16.0), &text, Color32::WHITE);
            }

            self.banner(ui);
        });

        if ctx.input(|i| {
            i.key_pressed(Key::Escape)
                || i.pointer.secondary_clicked()
                || i.viewport().close_requested()
        }) {
            outcome = SelectorOutcome::Cancelled;
        }

        // No timed repaint while idle: egui repaints on every mouse/keyboard event.
        if let Some(until) = self.too_small_until {
            let now = Instant::now();
            if now < until {
                ctx.request_repaint_after(until - now);
            } else {
                self.too_small_until = None;
            }
        }
        outcome
    }

    /// Instructions at the top (larger than body text), plus the "too small" message.
    fn banner(&self, ui: &mut Ui) {
        Frame::NONE
            .fill(Color32::from_black_alpha(200))
            .inner_margin(Margin::symmetric(16, 10))
            .corner_radius(6)
            .outer_margin(Margin::same(12))
            .show(ui, |ui| {
                let size = ui.text_style_height(&TextStyle::Heading) * 0.85;
                ui.label(
                    RichText::new(self.strings.selector_hint)
                        .size(size)
                        .color(Color32::WHITE),
                );
                if self.too_small_until.is_some() {
                    ui.label(
                        RichText::new(self.strings.selector_too_small)
                            .size(size)
                            .color(Color32::from_rgb(255, 209, 102))
                            .strong(),
                    );
                }
            });
    }
}

/// Text on a dark rounded background, kept inside the window.
fn draw_label(painter: &Painter, ui: &Ui, pos: Pos2, text: &str, color: Color32) {
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let size = galley.size() + vec2(16.0, 8.0);
    let bounds = painter.clip_rect();
    // Flip to the other side of the cursor near the right/bottom edges.
    let x = if pos.x + size.x > bounds.right() {
        pos.x - size.x - 32.0
    } else {
        pos.x
    };
    let y = if pos.y + size.y > bounds.bottom() {
        pos.y - size.y - 32.0
    } else {
        pos.y
    };
    let rect = Rect::from_min_size(pos2(x, y), size);
    painter.rect_filled(rect, 6, Color32::from_black_alpha(210));
    painter.galley(rect.min + vec2(8.0, 4.0), galley, color);
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
