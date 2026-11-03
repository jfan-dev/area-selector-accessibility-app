//! Screen capture with Win32 GDI (`BitBlt` from the desktop DC).

use eframe::egui::{Color32, ColorImage};
use windows::Win32::Graphics::Gdi::*;

/// Reusable capturer for a fixed-size region. The memory DC, bitmap and pixel buffer
/// are created once and reused every frame; all GDI handles are released on drop.
pub struct ScreenCapture {
    width: u32,
    height: u32,
    mem_dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bgra: Vec<u8>,
}

impl ScreenCapture {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        unsafe {
            let screen = ScreenDc::get()?;
            let mem_dc = CreateCompatibleDC(Some(screen.0));
            if mem_dc.is_invalid() {
                return None;
            }
            let bitmap = CreateCompatibleBitmap(screen.0, width as i32, height as i32);
            if bitmap.is_invalid() {
                let _ = DeleteDC(mem_dc);
                return None;
            }
            let previous = SelectObject(mem_dc, bitmap.into());
            Some(Self {
                width,
                height,
                mem_dc,
                bitmap,
                previous,
                bgra: vec![0; width as usize * height as usize * 4],
            })
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Captures the region whose top-left corner is `(x, y)` (physical pixels).
    pub fn grab(&mut self, x: i32, y: i32) -> Option<ColorImage> {
        unsafe {
            let screen = ScreenDc::get()?;
            let (w, h) = (self.width as i32, self.height as i32);
            BitBlt(self.mem_dc, 0, 0, w, h, Some(screen.0), x, y, SRCCOPY).ok()?;

            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down rows
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let rows = GetDIBits(
                self.mem_dc,
                self.bitmap,
                0,
                self.height,
                Some(self.bgra.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            );
            if rows == 0 {
                return None;
            }
        }
        Some(ColorImage::new(
            [self.width as usize, self.height as usize],
            bgra_to_color32(&self.bgra),
        ))
    }
}

// SAFETY: the memory DC and bitmap are GDI objects owned exclusively by this struct.
// Unlike window handles they have no thread affinity; they only must not be used from
// two threads at once, which `&mut self` (and the Mutex around the magnifier) prevents.
// In practice eframe runs every viewport callback on the main thread.
unsafe impl Send for ScreenCapture {}

impl Drop for ScreenCapture {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem_dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.mem_dc);
        }
    }
}

/// The desktop DC, released when dropped. Fetched per capture: holding a common
/// DC across frames is discouraged by the Win32 docs and getting it is cheap.
struct ScreenDc(HDC);

impl ScreenDc {
    unsafe fn get() -> Option<Self> {
        let dc = GetDC(None);
        (!dc.is_invalid()).then_some(Self(dc))
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}

/// Converts GDI's BGRX pixels to opaque egui colors in a single pass.
fn bgra_to_color32(bgra: &[u8]) -> Vec<Color32> {
    let (pixels, _) = bgra.as_chunks::<4>();
    pixels
        .iter()
        .map(|&[b, g, r, _]| Color32::from_rgb(r, g, b))
        .collect()
}

/// One-shot capture (used for the selector's background screenshot).
pub fn capture_area(x: u32, y: u32, w: u32, h: u32) -> Option<ColorImage> {
    ScreenCapture::new(w, h)?.grab(x as i32, y as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_to_opaque_rgb() {
        let bgra = [10, 20, 30, 0, 255, 128, 0, 77];
        assert_eq!(
            bgra_to_color32(&bgra),
            vec![
                Color32::from_rgb(30, 20, 10),
                Color32::from_rgb(0, 128, 255)
            ]
        );
    }

    /// Runs real GDI calls (tests run as a Windows process, also from WSL).
    #[test]
    fn captures_a_real_screen_region_repeatedly() {
        let mut capture = ScreenCapture::new(64, 32).expect("create capturer");
        for _ in 0..3 {
            let image = capture.grab(0, 0).expect("grab screen region");
            assert_eq!(image.size, [64, 32]);
            assert_eq!(image.pixels.len(), 64 * 32);
            assert!(
                image.pixels.iter().all(|p| p.a() == 255),
                "pixels are opaque"
            );
        }
        let one_shot = capture_area(10, 10, 8, 8).expect("one-shot capture");
        assert_eq!(one_shot.size, [8, 8]);
    }

    #[test]
    fn zero_size_capture_is_rejected() {
        assert!(ScreenCapture::new(0, 10).is_none());
        assert!(ScreenCapture::new(10, 0).is_none());
    }
}
