use eframe::egui::{self, *};
use crate::model::Area;
use crate::config::{self, AppConfig};
use std::process::{Command, Child};

pub struct App {
    saved_area: Option<Area>,
    seletor_child: Option<Child>,
    config: AppConfig,
}

impl Default for App {
    fn default() -> Self {
        let cfg = config::load_config();
        Self {
            saved_area: cfg.area,
            seletor_child: None,
            config: cfg,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    fn area_to_xywh(area: crate::model::Area) -> [String; 4] {
        let x = area.start.x.min(area.end.x).round() as u32;
        let y = area.start.y.min(area.end.y).round() as u32;
        let w = (area.start.x - area.end.x).abs().round() as u32;
        let h = (area.start.y - area.end.y).abs().round() as u32;
        [x.to_string(), y.to_string(), w.to_string(), h.to_string()]
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(child) = self.seletor_child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let cfg = config::load_config();
                    self.saved_area = cfg.area;
                    self.config = cfg;
                    self.seletor_child = None;
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Erro ao esperar seletor: {}", e);
                    self.seletor_child = None;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("🖥️ Área Selector");

                if ui.button("Selecionar Área").clicked() {
                    let exe = std::env::current_exe()
                        .unwrap()
                        .with_file_name("seletor.exe");
                    if let Ok(child) = Command::new(exe).spawn() {
                        self.seletor_child = Some(child);
                    }
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.add_enabled(self.saved_area.is_some(), Button::new("Iniciar Lupa")).clicked() {
                        if let Some(area) = self.saved_area {
                            let [x, y, w, h] = Self::area_to_xywh(area);
                            let exe = std::env::current_exe().unwrap().with_file_name("lupa.exe");
                            let zoom = self.config.lupa_zoom.to_string();
                            let pos_x = self.config.lupa_pos_x.to_string();
                            let pos_y = self.config.lupa_pos_y.to_string();

                            let _ = Command::new(&exe)
                                .args([x, y, w, h, zoom, pos_x, pos_y])
                                .spawn();
                        }
                    }

                    if ui.button("Alternar Lupa Passthrough").clicked() {
                        if let Some(area) = self.saved_area {
                            let [x, y, w, h] = Self::area_to_xywh(area);
                            let exe = std::env::current_exe().unwrap().with_file_name("lupa.exe");

                            let hwnd_path = dirs::config_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join("area_selector")
                                .join("lupa_hwnd.txt");

                            let mut passthrough = false;
                            let mut hwnd_val: Option<isize> = None;

                            if let Ok(data) = std::fs::read_to_string(&hwnd_path) {
                                if let Ok(val) = usize::from_str_radix(data.trim(), 10) {
                                    use windows::Win32::UI::WindowsAndMessaging::*;
                                    unsafe {
                                        let hwnd = windows::Win32::Foundation::HWND(val as *mut _);
                                        let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                                        hwnd_val = Some(val as isize);
                                        if style & WS_EX_TRANSPARENT.0 as i32 != 0 {
                                            passthrough = true;
                                        }
                                    }
                                }
                            }

                            let new_mode = if passthrough { "normal" } else { "passthrough" };

                            if let Some(val) = hwnd_val {
                                use windows::Win32::UI::WindowsAndMessaging::*;
                                unsafe {
                                    let hwnd = windows::Win32::Foundation::HWND(val as *mut _);
                                    let _ = SendMessageW(
                                        hwnd,
                                        WM_CLOSE,
                                        windows::Win32::Foundation::WPARAM(0),
                                        windows::Win32::Foundation::LPARAM(0),
                                    );
                                }
                                std::thread::sleep(std::time::Duration::from_millis(200));
                            }

                            let zoom = self.config.lupa_zoom.to_string();
                            let pos_x = self.config.lupa_pos_x.to_string();
                            let pos_y = self.config.lupa_pos_y.to_string();

                            if new_mode == "passthrough" {
                                let _ = std::process::Command::new(&exe)
                                    .args([x, y, w, h, "passthrough".to_string(), pos_x, pos_y])
                                    .spawn();
                            } else {
                                let _ = std::process::Command::new(&exe)
                                    .args([x, y, w, h, zoom, pos_x, pos_y])
                                    .spawn();
                            }
                        }
                    }
                });

                ui.separator();

                ui.heading("Configurações da Lupa");
                ui.add(Slider::new(&mut self.config.lupa_zoom, 0.5..=5.0).text("Zoom"));
                ui.add(Slider::new(&mut self.config.lupa_pos_x, 0.0..=1920.0).text("Posição X"));
                ui.add(Slider::new(&mut self.config.lupa_pos_y, 0.0..=1080.0).text("Posição Y"));

                if ui.button("💾 Salvar Configurações").clicked() {
                    config::save_config(&self.config);
                    ui.label("✔️ Configurações salvas!");
                }

                ui.separator();

                if let Some(area) = self.saved_area {
                    ui.label(format!(
                        "Área salva: start=({:.1}, {:.1}), end=({:.1}, {:.1})",
                        area.start.x, area.start.y, area.end.x, area.end.y
                    ));
                } else {
                    ui.label("Nenhuma área salva ainda.");
                }

                ui.separator();

                if ui.button("❌ Sair").clicked() {
                    std::process::exit(0);
                }
            });
        });
    }
}
