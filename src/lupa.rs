use crate::capture::ScreenCapture;
use crate::config::{BorderPreset, LUPA_ZOOM_RANGE};
use crate::i18n::{self, Strings};
use crate::{config, icon};
use eframe::egui::{
    self, vec2, Align2, Color32, CornerRadius, Event, FontId, Frame, Key, MouseWheelUnit, Sense,
    Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2, ViewportBuilder, ViewportCommand,
};
use std::time::Duration;

/// Screen region magnified by the lupa: `(x, y, width, height)` in physical pixels.
pub type CaptureRect = (u32, u32, u32, u32);

/// Zoom change per mouse-wheel notch or +/− key press.
pub const ZOOM_STEP: f32 = 0.25;
/// Trackpad scrolling (in points) that counts as one wheel notch.
const POINTS_PER_WHEEL_STEP: f32 = 50.0;

/// Result of one frame of the magnifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LupaOutcome {
    Open,
    /// The user pressed ESC (interactive mode) or the window was closed.
    Closed,
}

/// Changes made directly on the magnifier (drag, wheel, keys) that the panel should
/// copy into the settings. Taken with [`LupaApp::take_user_changes`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct UserChanges {
    pub zoom: Option<f32>,
    pub position: Option<[f32; 2]>,
    pub passthrough: Option<bool>,
}

/// Magnifier window state. All fields that affect the window (zoom, position,
/// passthrough, frame rate) can be changed while it is open; the next frame applies them.
///
/// Changes made on the magnifier itself are applied by the magnifier with its own window
/// commands, so they work even while the panel is minimized, and reported through
/// [`UserChanges`].
pub struct LupaApp {
    pub area: CaptureRect,
    /// True magnification: 2.0 shows every captured pixel as 2×2 screen pixels,
    /// whatever the Windows display scaling is.
    pub zoom: f32,
    /// Window position in points.
    pub position: [f32; 2],
    /// When true, mouse clicks go through the window to whatever is underneath.
    pub passthrough: bool,
    pub frame_interval: Duration,
    /// Border colours `(interactive, passthrough)` and thickness in points.
    pub border_colors: (Color32, Color32),
    pub border_width: f32,
    /// Show the interactive/passthrough badge (state is never shown by colour alone).
    pub show_mode_badge: bool,
    /// Texts (loading, hover hint, badge).
    pub strings: &'static Strings,
    /// Display scale of the magnifier's monitor, refreshed every frame.
    pixels_per_point: f32,
    capture: Option<ScreenCapture>,
    texture: Option<TextureHandle>,
    excluded_from_capture: bool,
    changes: UserChanges,
    /// After a drag started, follow the window position the OS reports.
    following_drag: bool,
    wheel_accumulator: f32,
}

impl LupaApp {
    pub const TITLE: &'static str = "Lupa";

    pub fn new(
        area: CaptureRect,
        zoom: f32,
        position: [f32; 2],
        passthrough: bool,
        pixels_per_point: f32,
    ) -> Self {
        Self {
            area,
            zoom,
            position,
            passthrough,
            frame_interval: Duration::from_secs(1) / config::DEFAULT_LUPA_FPS,
            border_colors: BorderPreset::default().colors(),
            border_width: 3.0,
            show_mode_badge: true,
            strings: &i18n::PT,
            pixels_per_point,
            capture: None,
            texture: None,
            excluded_from_capture: false,
            changes: UserChanges::default(),
            following_drag: false,
            wheel_accumulator: 0.0,
        }
    }

    /// Changes made on the magnifier since the last call.
    pub fn take_user_changes(&mut self) -> UserChanges {
        std::mem::take(&mut self.changes)
    }

    /// Switches to a new capture area. The capture buffers are rebuilt for the new size on
    /// the next frame, and the window resizes through the next viewport builder.
    pub fn set_area(&mut self, area: CaptureRect) {
        self.area = area;
    }

    /// Moves the window to `position` (set by the panel). Stops following a drag.
    pub fn set_position(&mut self, position: [f32; 2]) {
        if position != self.position {
            self.position = position;
            self.following_drag = false;
        }
    }

    /// Size of the magnified image in points.
    fn display_size(&self) -> Vec2 {
        display_size(self.area, self.zoom, self.pixels_per_point)
    }

    /// Borderless, transparent, always-on-top window sized to `area × zoom`.
    /// Re-sent every frame: egui only issues window commands for fields that changed.
    pub fn viewport_builder(&self) -> ViewportBuilder {
        let builder = match icon::app_icon() {
            Some(icon) => ViewportBuilder::default().with_icon(icon),
            None => ViewportBuilder::default(),
        };
        builder
            .with_title(Self::TITLE)
            .with_inner_size(self.display_size())
            .with_position(self.position)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(self.passthrough)
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let (x, y, w, h) = self.area;
        if self.capture.as_ref().map(ScreenCapture::size) != Some((w, h)) {
            self.capture = ScreenCapture::new(w, h);
        }
        let Some(image) = self
            .capture
            .as_mut()
            .and_then(|c| c.grab(x as i32, y as i32))
        else {
            return;
        };

        match &mut self.texture {
            Some(tex) => tex.set(image, TextureOptions::LINEAR),
            None => {
                self.texture = Some(ctx.load_texture("lupa_texture", image, TextureOptions::LINEAR))
            }
        }
    }

    fn zoom_by_user(&mut self, ctx: &egui::Context, steps: i32) {
        let zoom = step_zoom(self.zoom, steps);
        if zoom != self.zoom {
            self.zoom = zoom;
            self.changes.zoom = Some(zoom);
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(self.display_size()));
        }
    }

    fn move_by_user(&mut self, ctx: &egui::Context, delta: Vec2) {
        self.position = [self.position[0] + delta.x, self.position[1] + delta.y];
        self.changes.position = Some(self.position);
        self.following_drag = false;
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(self.position.into()));
    }

    /// Wheel, keyboard and drag-follow handling (interactive mode only: a passthrough
    /// window receives no input).
    fn handle_input(&mut self, ctx: &egui::Context) {
        if self.passthrough {
            return;
        }
        let (events, shift) = ctx.input(|i| (i.events.clone(), i.modifiers.shift));
        let mut zoom_steps = 0;
        let mut movement = Vec2::ZERO;
        let move_step = if shift { 10.0 } else { 1.0 };
        for event in &events {
            match event {
                Event::MouseWheel { unit, delta, .. } => {
                    zoom_steps += wheel_steps(&mut self.wheel_accumulator, *unit, delta.y);
                }
                Event::Key {
                    key, pressed: true, ..
                } => match key {
                    Key::Plus | Key::Equals => zoom_steps += 1,
                    Key::Minus => zoom_steps -= 1,
                    Key::ArrowLeft => movement.x -= move_step,
                    Key::ArrowRight => movement.x += move_step,
                    Key::ArrowUp => movement.y -= move_step,
                    Key::ArrowDown => movement.y += move_step,
                    Key::P => {
                        self.passthrough = true;
                        self.changes.passthrough = Some(true);
                        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if zoom_steps != 0 {
            self.zoom_by_user(ctx, zoom_steps);
        }
        if movement != Vec2::ZERO {
            self.move_by_user(ctx, movement);
        }

        // While/after an OS drag, adopt the position the window actually has.
        if self.following_drag {
            if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                let position = [outer.min.x.round(), outer.min.y.round()];
                if position != self.position {
                    self.position = position;
                    self.changes.position = Some(position);
                }
            }
        }
    }

    /// Captures the area, draws it zoomed, and reports whether the window should close.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> LupaOutcome {
        let ctx = &ui.ctx().clone();
        self.pixels_per_point = ctx.pixels_per_point();
        if !self.excluded_from_capture {
            self.excluded_from_capture = exclude_own_window_from_capture(Self::TITLE);
        }
        self.handle_input(ctx);
        self.update_texture(ctx);

        let display_size = self.display_size();
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);

                if let Some(tex) = &self.texture {
                    ui.image((tex.id(), display_size));
                } else {
                    ui.label(self.strings.lupa_loading);
                }

                // Drag anywhere to move the window (interactive mode).
                let response = ui.interact(
                    rect,
                    ui.scope_id().with("lupa_drag"),
                    Sense::click_and_drag(),
                );
                if response.drag_started() && !self.passthrough {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    self.following_drag = true;
                }

                let (interactive, passthrough) = self.border_colors;
                let color = if self.passthrough {
                    passthrough
                } else {
                    interactive
                };
                painter.rect_stroke(
                    rect,
                    CornerRadius::ZERO,
                    Stroke::new(self.border_width, color),
                    StrokeKind::Middle,
                );

                // Badge and hint follow the panel's text size setting.
                let small = egui::TextStyle::Small.resolve(ui.style());
                if self.show_mode_badge {
                    let text = if self.passthrough {
                        self.strings.badge_passthrough
                    } else {
                        self.strings.badge_interactive
                    };
                    let inset = self.border_width + 4.0;
                    draw_badge(
                        &painter,
                        rect.min + vec2(inset, inset),
                        text,
                        color,
                        small.clone(),
                    );
                }

                if response.hovered() && !self.passthrough {
                    let inset = self.border_width + 4.0;
                    draw_hint(
                        &painter,
                        rect.center_bottom() - vec2(0.0, inset),
                        rect.width() - 2.0 * inset,
                        self.strings.lupa_hint,
                        small,
                    );
                }
            });

        ctx.request_repaint_after(self.frame_interval);

        if ctx.input(|i| i.key_pressed(Key::Escape) || i.viewport().close_requested()) {
            LupaOutcome::Closed
        } else {
            LupaOutcome::Open
        }
    }
}

/// Small label in the top-left corner: the mode in words, framed in the border colour.
fn draw_badge(
    painter: &egui::Painter,
    top_left: egui::Pos2,
    text: &str,
    color: Color32,
    font: FontId,
) {
    let galley = painter.layout_no_wrap(text.to_owned(), font, Color32::WHITE);
    let rect = egui::Rect::from_min_size(top_left, galley.size() + vec2(12.0, 6.0));
    painter.rect(
        rect,
        4,
        Color32::from_black_alpha(200),
        Stroke::new(2.0, color),
        StrokeKind::Inside,
    );
    painter.galley(rect.min + vec2(6.0, 3.0), galley, Color32::WHITE);
}

/// Usage hint at the bottom while the mouse is over an interactive magnifier.
fn draw_hint(
    painter: &egui::Painter,
    bottom_center: egui::Pos2,
    max_width: f32,
    text: &str,
    font: FontId,
) {
    let galley = painter.layout(text.to_owned(), font, Color32::WHITE, max_width.max(60.0));
    let size = galley.size() + vec2(12.0, 6.0);
    let rect = Align2::CENTER_BOTTOM.anchor_size(bottom_center, size);
    painter.rect_filled(rect, 4, Color32::from_black_alpha(190));
    painter.galley(rect.min + vec2(6.0, 3.0), galley, Color32::WHITE);
}

/// Zoom after `steps` notches (positive = in), kept in [`LUPA_ZOOM_RANGE`].
pub fn step_zoom(zoom: f32, steps: i32) -> f32 {
    let (min, max) = (*LUPA_ZOOM_RANGE.start(), *LUPA_ZOOM_RANGE.end());
    (zoom + steps as f32 * ZOOM_STEP).clamp(min, max)
}

/// Converts one mouse-wheel event to zoom steps (wheel up = zoom in). A mouse sends
/// one event per notch (lines); trackpads send many small ones (points), which are
/// accumulated so one step needs [`POINTS_PER_WHEEL_STEP`] of scrolling.
pub fn wheel_steps(accumulator: &mut f32, unit: MouseWheelUnit, delta_y: f32) -> i32 {
    match unit {
        MouseWheelUnit::Line | MouseWheelUnit::Page => {
            *accumulator = 0.0;
            if delta_y > 0.0 {
                1
            } else if delta_y < 0.0 {
                -1
            } else {
                0
            }
        }
        MouseWheelUnit::Point => {
            *accumulator += delta_y;
            let steps = (*accumulator / POINTS_PER_WHEEL_STEP).trunc();
            *accumulator -= steps * POINTS_PER_WHEEL_STEP;
            steps as i32
        }
    }
}

/// Size in points of the magnifier window for `area` (physical pixels) at `zoom`.
pub fn display_size(area: CaptureRect, zoom: f32, pixels_per_point: f32) -> Vec2 {
    let (_, _, w, h) = area;
    egui::vec2(w as f32, h as f32) * zoom / pixels_per_point
}

/// Quick placements for the magnifier window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    TopLeft,
    TopRight,
    Center,
    BottomLeft,
    BottomRight,
}

impl Placement {
    /// Gap to the screen edges, in points.
    pub const MARGIN: f32 = 16.0;
    /// Extra gap at the bottom so the magnifier doesn't sit under the taskbar.
    pub const BOTTOM_MARGIN: f32 = 64.0;

    /// Top-left window position for a window of `size` on a screen of `screen` (points).
    /// Windows larger than the screen are placed at the top-left margin.
    pub fn position(self, screen: Vec2, size: Vec2) -> [f32; 2] {
        let left = Self::MARGIN;
        let top = Self::MARGIN;
        let right = (screen.x - size.x - Self::MARGIN).max(left);
        let bottom = (screen.y - size.y - Self::BOTTOM_MARGIN).max(top);
        let center_x = ((screen.x - size.x) / 2.0).max(left);
        let center_y = ((screen.y - size.y) / 2.0).max(top);
        match self {
            Self::TopLeft => [left, top],
            Self::TopRight => [right, top],
            Self::Center => [center_x, center_y],
            Self::BottomLeft => [left, bottom],
            Self::BottomRight => [right, bottom],
        }
    }
}

/// Hides this process's window titled `title` from screen captures
/// (`WDA_EXCLUDEFROMCAPTURE`, Windows 10 2004+), so the magnifier never magnifies
/// itself when it overlaps the selected area. Only windows of this process are
/// searched — the built-in Windows Magnifier is also called "Lupa" in Portuguese.
/// Returns true once the window was found and updated.
fn exclude_own_window_from_capture(title: &str) -> bool {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, SetWindowDisplayAffinity,
        WDA_EXCLUDEFROMCAPTURE,
    };

    struct Search {
        title: Vec<u16>,
        pid: u32,
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != search.pid {
            return true.into();
        }
        let mut buf = [0u16; 64];
        let len = GetWindowTextW(hwnd, &mut buf) as usize;
        if buf[..len] == search.title[..] {
            search.found = Some(hwnd);
            return false.into(); // stop enumerating
        }
        true.into()
    }

    let mut search = Search {
        title: title.encode_utf16().collect(),
        pid: unsafe { GetCurrentProcessId() },
        found: None,
    };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize));
    }
    let Some(hwnd) = search.found else {
        return false; // window not created yet; try again next frame
    };
    if let Err(err) = unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) } {
        log::warn!("could not exclude the magnifier from capture: {err}");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    #[test]
    fn zoom_steps_are_clamped() {
        assert_eq!(step_zoom(1.0, 1), 1.25);
        assert_eq!(step_zoom(1.0, -2), 0.5);
        assert_eq!(step_zoom(4.9, 3), 5.0);
        assert_eq!(step_zoom(0.6, -5), 0.5);
    }

    #[test]
    fn mouse_wheel_notches_are_one_step_each() {
        let mut acc = 0.0;
        assert_eq!(wheel_steps(&mut acc, MouseWheelUnit::Line, 3.0), 1);
        assert_eq!(wheel_steps(&mut acc, MouseWheelUnit::Line, -1.0), -1);
    }

    #[test]
    fn trackpad_scrolling_is_accumulated() {
        let mut acc = 0.0;
        assert_eq!(wheel_steps(&mut acc, MouseWheelUnit::Point, 20.0), 0);
        assert_eq!(wheel_steps(&mut acc, MouseWheelUnit::Point, 20.0), 0);
        assert_eq!(
            wheel_steps(&mut acc, MouseWheelUnit::Point, 20.0),
            1,
            "60 points"
        );
        assert_eq!(wheel_steps(&mut acc, MouseWheelUnit::Point, -120.0), -2);
    }

    #[test]
    fn display_size_accounts_for_zoom_and_scaling() {
        assert_eq!(display_size((0, 0, 300, 200), 2.0, 1.0), vec2(600.0, 400.0));
        assert_eq!(
            display_size((0, 0, 300, 200), 2.0, 1.5),
            vec2(400.0, 800.0 / 3.0)
        );
    }

    #[test]
    fn placements_keep_the_window_on_screen() {
        let screen = vec2(1920.0, 1080.0);
        let size = vec2(400.0, 300.0);
        assert_eq!(Placement::TopLeft.position(screen, size), [16.0, 16.0]);
        assert_eq!(Placement::TopRight.position(screen, size), [1504.0, 16.0]);
        assert_eq!(Placement::Center.position(screen, size), [760.0, 390.0]);
        assert_eq!(Placement::BottomLeft.position(screen, size), [16.0, 716.0]);
        assert_eq!(
            Placement::BottomRight.position(screen, size),
            [1504.0, 716.0]
        );
    }

    #[test]
    fn oversized_window_is_pinned_to_the_top_left_margin() {
        let screen = vec2(800.0, 600.0);
        let size = vec2(1000.0, 900.0);
        for placement in [
            Placement::TopLeft,
            Placement::TopRight,
            Placement::Center,
            Placement::BottomLeft,
            Placement::BottomRight,
        ] {
            assert_eq!(placement.position(screen, size), [16.0, 16.0]);
        }
    }
}
