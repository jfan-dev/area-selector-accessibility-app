use crate::capture::ScreenCapture;
use crate::config::BorderPreset;
use crate::{config, i18n, icon};
use eframe::egui::{self, Color32, Frame, TextureHandle, TextureOptions, Vec2, ViewportBuilder};
use std::time::Duration;

/// Screen region magnified by the lupa: `(x, y, width, height)` in physical pixels.
pub type CaptureRect = (u32, u32, u32, u32);

/// Result of one frame of the magnifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LupaOutcome {
    Open,
    /// The user pressed ESC (interactive mode) or the window was closed.
    Closed,
}

/// Magnifier window state. All fields that affect the window (zoom, position,
/// passthrough, frame rate) can be changed while it is open; the next frame applies them.
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
    /// Shown until the first frame has been captured.
    pub loading_text: &'static str,
    /// Display scale of the magnifier's monitor, refreshed every frame.
    pixels_per_point: f32,
    capture: Option<ScreenCapture>,
    texture: Option<TextureHandle>,
    excluded_from_capture: bool,
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
            loading_text: i18n::PT.lupa_loading,
            pixels_per_point,
            capture: None,
            texture: None,
            excluded_from_capture: false,
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

    /// Captures the area, draws it zoomed, and reports whether the window should close.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> LupaOutcome {
        let ctx = &ui.ctx().clone();
        self.pixels_per_point = ctx.pixels_per_point();
        if !self.excluded_from_capture {
            self.excluded_from_capture = exclude_own_window_from_capture(Self::TITLE);
        }
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
                    ui.label(self.loading_text);
                }

                let (interactive, passthrough) = self.border_colors;
                let color = if self.passthrough {
                    passthrough
                } else {
                    interactive
                };
                painter.rect_stroke(
                    rect,
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(self.border_width, color),
                    egui::StrokeKind::Middle,
                );
            });

        ctx.request_repaint_after(self.frame_interval);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            LupaOutcome::Closed
        } else {
            LupaOutcome::Open
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
