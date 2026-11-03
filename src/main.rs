// Release builds are GUI-only (no console window). Debug builds keep the console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::App;
use area_selector::cli::{self, LupaArgs, Mode};
use area_selector::lupa::{LupaApp, LupaOutcome};
use area_selector::ui_selector::{self, AreaSelector, SelectorOutcome};
use area_selector::{config, icon};
use eframe::egui::{self, ViewportBuilder, ViewportCommand};

fn main() {
    attach_parent_console();
    enable_dpi_awareness();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match cli::parse(&args) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("Erro: {err}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
    };

    let result = match mode {
        Mode::Panel => run_panel(),
        Mode::Selector => run_selector(),
        Mode::Lupa(lupa) => run_lupa(lupa),
    };

    if let Err(err) = result {
        eprintln!("Erro: {err}");
        std::process::exit(1);
    }
}

fn run_panel() -> eframe::Result<()> {
    let mut viewport = ViewportBuilder::default();
    if let Some(icon) = icon::app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_inner_size([400.0, 200.0])
            .with_resizable(false)
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };
    eframe::run_native(
        "Selecionador de Área",
        options,
        Box::new(|_cc| Box::new(App::new())),
    )
}

/// `area_selector seletor`: only the fullscreen selector; saves the area and exits.
fn run_selector() -> eframe::Result<()> {
    struct SelectorRoot(AreaSelector);

    impl eframe::App for SelectorRoot {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            match self.0.ui(ctx) {
                SelectorOutcome::Pending => {}
                SelectorOutcome::Selected(area) => {
                    config::save_area(&area);
                    println!("✅ Área salva: {:?}", area);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                SelectorOutcome::Cancelled => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    // Screenshot is taken before the overlay window exists.
    let selector = AreaSelector::capture_screen();
    let options = eframe::NativeOptions {
        viewport: AreaSelector::viewport_builder(ui_selector::primary_screen_size_px()),
        ..Default::default()
    };
    eframe::run_native(
        AreaSelector::TITLE,
        options,
        Box::new(|_cc| Box::new(SelectorRoot(selector))),
    )
}

/// `area_selector lupa ...`: only the magnifier window.
fn run_lupa(args: LupaArgs) -> eframe::Result<()> {
    struct LupaRoot(LupaApp);

    impl eframe::App for LupaRoot {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            if self.0.ui(ctx) == LupaOutcome::Closed {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    let mut cfg = config::load_config();
    if let Some([x, y]) = args.position {
        cfg.lupa_pos_x = x;
        cfg.lupa_pos_y = y;
        config::save_config(&cfg);
        println!("💾 Nova posição salva: ({x}, {y})");
    }

    let lupa = LupaApp::new(
        (args.x, args.y, args.w, args.h),
        args.zoom.unwrap_or(cfg.lupa_zoom),
        [cfg.lupa_pos_x, cfg.lupa_pos_y],
        args.passthrough,
    );
    let options = eframe::NativeOptions {
        viewport: lupa.viewport_builder(),
        ..Default::default()
    };
    eframe::run_native(
        LupaApp::TITLE,
        options,
        Box::new(move |_cc| Box::new(LupaRoot(lupa))),
    )
}

/// Makes screen metrics and screenshots use physical pixels from the start, so the
/// selector screenshot is correct on scaled displays even before winit starts
/// (winit would set the same awareness later; a second call just fails harmlessly).
fn enable_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Without a console window (release builds), CLI messages such as usage errors would be
/// lost. When started from a terminal, write them to that terminal instead. When started
/// by double-click there is no parent console and this does nothing.
fn attach_parent_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
