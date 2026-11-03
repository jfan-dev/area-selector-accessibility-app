//! Application icon, embedded in the binary so the exe stays standalone.

use eframe::egui::IconData;
use std::sync::{Arc, OnceLock};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Window/taskbar icon size. The source PNG is 1024×1024; Windows never shows more than 256.
const ICON_SIZE: u32 = 256;

/// Decoded once, shared by every window.
pub fn app_icon() -> Option<Arc<IconData>> {
    static ICON: OnceLock<Option<Arc<IconData>>> = OnceLock::new();
    ICON.get_or_init(|| decode(ICON_PNG, ICON_SIZE).map(Arc::new))
        .clone()
}

/// The icon as RGBA pixels at `size`×`size` (e.g. 32 for the system tray).
pub fn app_icon_rgba(size: u32) -> Option<IconData> {
    decode(ICON_PNG, size)
}

fn decode(png: &[u8], size: u32) -> Option<IconData> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let image = image
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    Some(IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes() {
        let icon = app_icon().expect("embedded icon should decode");
        assert_eq!((icon.width, icon.height), (ICON_SIZE, ICON_SIZE));
        assert_eq!(icon.rgba.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    }
}
