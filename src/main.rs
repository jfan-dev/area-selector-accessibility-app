mod app;

use app::App;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Selecionador de Área",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size([400.0, 200.0])
                .with_resizable(false)
                .with_decorations(true)
                .with_transparent(false),
            ..Default::default()
        },
        Box::new(|_cc| Box::new(App::new())),
    )
}
