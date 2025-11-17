use area_selector::{lupa::LupaApp, config};
use eframe::egui::{self, ViewportBuilder};
use std::{fs, path::PathBuf, thread, time::Duration};
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

fn hwnd_file_path() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join("area_selector").join("lupa_hwnd.txt")
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 5 {
        eprintln!("Uso: lupa <x> <y> <w> <h> [zoom|passthrough] [pos_x pos_y]");
        std::process::exit(1);
    }

    let x = args[1].parse().unwrap_or(0);
    let y = args[2].parse().unwrap_or(0);
    let w = args[3].parse().unwrap_or(100);
    let h = args[4].parse().unwrap_or(100);

    let mut cfg = config::load_config();
    let mut zoom = cfg.lupa_zoom;
    let mut passthrough = false;

    if args.len() >= 6 {
        if args[5].eq_ignore_ascii_case("passthrough") {
            passthrough = true;
        } else {
            zoom = args[5].parse().unwrap_or(cfg.lupa_zoom);
        }
    }

    // posição opcional vinda dos argumentos
    if args.len() >= 8 {
        cfg.lupa_pos_x = args[6].parse::<f32>().unwrap_or(cfg.lupa_pos_x);
        cfg.lupa_pos_y = args[7].parse::<f32>().unwrap_or(cfg.lupa_pos_y);
        config::save_config(&cfg);
        println!("💾 Nova posição salva: ({}, {})", cfg.lupa_pos_x, cfg.lupa_pos_y);
    }

    let area = (x, y, w, h);

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([w as f32 * zoom, h as f32 * zoom])
            .with_position([cfg.lupa_pos_x, cfg.lupa_pos_y])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Lupa",
        options,
        Box::new(move |_cc| {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(500));
                unsafe {
                    if let Some(hwnd) = get_foreground_hwnd() {
                        save_hwnd(hwnd);
                        if passthrough {
                            set_passthrough(hwnd, true);
                        }
                    }
                }
            });

            Box::new(LupaWithExit::new(area, zoom, !passthrough))
        }),
    )
}

struct LupaWithExit {
    lupa: LupaApp,
}

impl LupaWithExit {
    fn new(area: (u32, u32, u32, u32), zoom: f32, interactive: bool) -> Self {
        Self { lupa: LupaApp::new(area, zoom, interactive) }
    }
}

impl eframe::App for LupaWithExit {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.lupa.update(ctx, frame);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            std::process::exit(0);
        }
    }
}

unsafe fn get_foreground_hwnd() -> Option<HWND> {
    let hwnd = GetForegroundWindow();
    if hwnd.0.is_null() { None } else { Some(hwnd) }
}

fn save_hwnd(hwnd: HWND) {
    if let Some(parent) = hwnd_file_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(hwnd_file_path(), format!("{}", hwnd.0 as usize));
}

fn load_hwnd() -> Option<HWND> {
    if let Ok(data) = fs::read_to_string(hwnd_file_path()) {
        if let Ok(val) = usize::from_str_radix(data.trim(), 10) {
            return Some(HWND(val as *mut _));
        }
    }
    None
}

unsafe fn set_passthrough(hwnd: HWND, enable: bool) {
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    let new_style = if enable {
        ex_style | WS_EX_LAYERED.0 as i32 | WS_EX_TRANSPARENT.0 as i32
    } else {
        ex_style & !(WS_EX_TRANSPARENT.0 as i32)
    };
    SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
}
