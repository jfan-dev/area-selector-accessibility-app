use area_selector::ui_selector::AreaSelector;
use eframe::{egui::ViewportBuilder, Renderer};

// Importações atualizadas para windows 0.58
use windows::Win32::UI::WindowsAndMessaging::{SM_CXSCREEN, SM_CYSCREEN, GetSystemMetrics};

fn main() -> eframe::Result<()> {
    // Captura tamanho da tela via API do Windows
    let screen_width = (unsafe { GetSystemMetrics(SM_CXSCREEN) } - 1) as f32;
    let screen_height = (unsafe { GetSystemMetrics(SM_CYSCREEN) }) as f32;

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([screen_width, screen_height])
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false),
        renderer: Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Selecionar Área",
        options,
        Box::new(|_cc| Box::new(AreaSelector::default())),
    )
}
