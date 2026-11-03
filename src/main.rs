// Release builds are GUI-only (no console window). Debug builds keep the console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::App;
use area_selector::cli::{self, LupaArgs, Mode};
use area_selector::config::AppConfig;
use area_selector::i18n::{self, Lang, Strings};
use area_selector::lupa::{LupaApp, LupaOutcome};
use area_selector::ui_selector::{self, AreaSelector, SelectorOutcome};
use area_selector::{config, icon, theme};
use eframe::egui::{self, ViewportBuilder, ViewportCommand};

fn main() {
    attach_parent_console();
    init_logging();
    enable_dpi_awareness();
    let cfg = config::load_config();
    let s = i18n::tr(Lang::resolve(cfg.language));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match cli::parse(&args) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("{}: {}\n\n{}", s.cli_error, err.message(s), s.cli_usage);
            std::process::exit(2);
        }
    };

    let result = match mode {
        Mode::Panel => run_panel(),
        Mode::Selector => run_selector(cfg, s),
        Mode::Lupa(lupa) => run_lupa(lupa, cfg, s),
    };

    if let Err(err) = result {
        eprintln!("{}: {err}", s.cli_error);
        std::process::exit(1);
    }
}

fn run_panel() -> eframe::Result {
    let mut viewport = ViewportBuilder::default();
    if let Some(icon) = icon::app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_inner_size([600.0, 720.0])
            .with_min_inner_size([320.0, 400.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };
    eframe::run_native(
        "Area Selector",
        options,
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))),
    )
}

/// `area_selector seletor`: only the fullscreen selector; saves the area and exits.
fn run_selector(cfg: AppConfig, s: &'static Strings) -> eframe::Result {
    struct SelectorRoot(AreaSelector, &'static Strings);

    impl eframe::App for SelectorRoot {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = &ui.ctx().clone();
            match self.0.ui(ui) {
                SelectorOutcome::Pending => {}
                SelectorOutcome::Selected(area) => {
                    let s = self.1;
                    let (x, y, w, h) = area.to_rect_px();
                    match config::save_area(&area) {
                        Ok(()) => println!("{}", (s.cli_area_saved)(w, h, x, y)),
                        Err(err) => eprintln!("{}", (s.cli_save_failed)(&err.to_string())),
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                SelectorOutcome::Cancelled => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    // Screenshot is taken before the overlay window exists.
    let selector = AreaSelector::capture_screen().with_hint(s.selector_hint);
    let options = eframe::NativeOptions {
        viewport: AreaSelector::viewport_builder(ui_selector::primary_screen_size_px()),
        ..Default::default()
    };
    eframe::run_native(
        AreaSelector::TITLE,
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &cfg.appearance);
            Ok(Box::new(SelectorRoot(selector, s)))
        }),
    )
}

/// `area_selector lupa ...`: only the magnifier window.
fn run_lupa(args: LupaArgs, mut cfg: AppConfig, s: &'static Strings) -> eframe::Result {
    struct LupaRoot(LupaApp);

    impl eframe::App for LupaRoot {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            if self.0.ui(ui) == LupaOutcome::Closed {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    if let Some([x, y]) = args.position {
        cfg.lupa_pos_x = x;
        cfg.lupa_pos_y = y;
        match config::save_config(&cfg) {
            Ok(()) => println!("{}", (s.cli_position_saved)(x, y)),
            Err(err) => eprintln!("{}", (s.cli_save_failed)(&err.to_string())),
        }
    }

    let mut lupa = LupaApp::new(
        (args.x, args.y, args.w, args.h),
        args.zoom.unwrap_or(cfg.lupa_zoom),
        [cfg.lupa_pos_x, cfg.lupa_pos_y],
        args.passthrough,
        system_pixels_per_point(),
    );
    lupa.frame_interval = cfg.lupa_frame_interval();
    lupa.border_colors = cfg.appearance.border_preset.colors();
    lupa.border_width = cfg.appearance.border_width;
    lupa.loading_text = s.lupa_loading;
    let options = eframe::NativeOptions {
        viewport: lupa.viewport_builder(),
        ..Default::default()
    };
    eframe::run_native(
        LupaApp::TITLE,
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &cfg.appearance);
            Ok(Box::new(LupaRoot(lupa)))
        }),
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

/// Logs go to stderr (visible when started from a terminal). Default level is `warn`;
/// override with e.g. `RUST_LOG=debug`.
fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
}

/// Display scale of the primary monitor, for sizing the standalone magnifier window
/// before it exists (e.g. 1.5 at 150% scaling).
fn system_pixels_per_point() -> f32 {
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForSystem() };
    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / 96.0
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
