//! Application icon, embedded in the binary so the exe stays standalone.

use eframe::egui::IconData;
use std::sync::{Arc, OnceLock};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Window/taskbar icon size. The source PNG is 1024×1024; Windows never shows more than 256.
const ICON_SIZE: u32 = 256;

/// Decoded once, shared by every window.
pub fn app_icon() -> Option<Arc<IconData>> {
    static ICON: OnceLock<Option<Arc<IconData>>> = OnceLock::new();
    ICON.get_or_init(|| decode(ICON_PNG).map(Arc::new)).clone()
}

fn decode(png: &[u8]) -> Option<IconData> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let image = image
        .resize_exact(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3)
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
