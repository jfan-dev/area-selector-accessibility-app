//! Embeds the app icon and version info into the Windows executable.
//!
//! The `.ico` is generated from `assets/icon.png` at build time (no binary icon file
//! in the repo), then compiled into the exe with a resource script.

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::imageops::FilterType;
use image::ColorType;
use std::{env, fs, path::Path, path::PathBuf};

const ICON_SIZES: [u32; 6] = [256, 64, 48, 32, 24, 16];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.png");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let ico_path = out_dir.join("app.ico");
    write_ico(Path::new("assets/icon.png"), &ico_path);

    let rc_path = out_dir.join("app.rc");
    fs::write(&rc_path, resource_script(&ico_path)).expect("write app.rc");

    embed_resource::compile(&rc_path, embed_resource::NONE);
}

fn write_ico(png_path: &Path, ico_path: &Path) {
    let source = image::open(png_path).expect("read assets/icon.png");
    let encoded: Vec<(Vec<u8>, u32)> = ICON_SIZES
        .iter()
        .map(|&size| {
            let rgba = source
                .resize_exact(size, size, FilterType::Lanczos3)
                .into_rgba8()
                .into_raw();
            (rgba, size)
        })
        .collect();
    let frames: Vec<IcoFrame> = encoded
        .iter()
        .map(|(rgba, size)| {
            IcoFrame::as_png(rgba, *size, *size, ColorType::Rgba8).expect("encode icon frame")
        })
        .collect();

    let file = fs::File::create(ico_path).expect("create app.ico");
    IcoEncoder::new(file)
        .encode_images(&frames)
        .expect("write app.ico");
}

fn resource_script(ico_path: &Path) -> String {
    let icon = ico_path.display().to_string().replace('\\', "/");
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is set by cargo");
    let mut parts = version
        .split(['.', '-'])
        .map(|p| p.parse::<u16>().unwrap_or(0));
    let (major, minor, patch) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );

    format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "jfan-dev"
      VALUE "FileDescription", "Area Selector - accessibility magnifier"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "area_selector"
      VALUE "OriginalFilename", "area_selector.exe"
      VALUE "ProductName", "Area Selector"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}
