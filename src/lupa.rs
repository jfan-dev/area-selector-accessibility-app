use crate::capture::ScreenCapture;
use crate::{config, icon};
use eframe::egui::{self, Frame, TextureHandle, TextureOptions, ViewportBuilder};
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
            pixels_per_point,
            capture: None,
            texture: None,
            excluded_from_capture: false,
        }
    }

    /// Size of the magnified image in points.
    fn display_size(&self) -> egui::Vec2 {
        let (_, _, w, h) = self.area;
        egui::vec2(w as f32, h as f32) * self.zoom / self.pixels_per_point
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
    pub fn ui(&mut self, ctx: &egui::Context) -> LupaOutcome {
        self.pixels_per_point = ctx.pixels_per_point();
        if !self.excluded_from_capture {
            self.excluded_from_capture = exclude_own_window_from_capture(Self::TITLE);
        }
        self.update_texture(ctx);

        let display_size = self.display_size();
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);

                if let Some(tex) = &self.texture {
                    ui.image((tex.id(), display_size));
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

        ctx.request_repaint_after(self.frame_interval);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            LupaOutcome::Closed
        } else {
            LupaOutcome::Open
        }
    }
}

/// Hides this process's window titled `title` from screen captures
/// (`WDA_EXCLUDEFROMCAPTURE`, Windows 10 2004+), so the magnifier never magnifies
/// itself when it overlaps the selected area. Only windows of this process are
/// searched — the built-in Windows Magnifier is also called "Lupa" in Portuguese.
/// Returns true once the window was found and updated.
fn exclude_own_window_from_capture(title: &str) -> bool {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
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
