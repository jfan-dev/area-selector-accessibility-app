//! Control panel UI: header with status chips, Home and Appearance tabs, footer.
//!
//! Layout rules for large text sizes: captions go above their control, sliders fill
//! the available width, and rows that may not fit use wrapping layouts.
//! Accessibility: every card and choice is a named group in the accessibility tree,
//! and symbol-only buttons have spoken names.

use super::{App, Tab};
use area_selector::config::{
    BorderPreset, ThemeMode, BORDER_WIDTH_RANGE, LUPA_FPS_RANGE, LUPA_ZOOM_RANGE, UI_SCALE_RANGE,
};
use area_selector::i18n::{Lang, Strings};
use area_selector::lupa::{self, Placement};
use area_selector::theme;
use area_selector::ui_selector;
use area_selector::widgets::{
    a11y_group, button_width, caption, card, chip_button, fit_slider_width, icon_button, icon_pad,
    radio_width, row_or_column, slider_value_fits, slider_with_caption, swatch, swatch_width,
    text_width, toggle_switch,
};
use eframe::egui::*;
use std::time::Instant;

const ZOOM_STEP: f32 = 0.25;
const UI_SCALE_STEP: f32 = 0.1;

impl App {
    pub(super) fn panel_ui(&mut self, ui: &mut Ui) {
        let s = self.strings();
        #[cfg(test)]
        {
            self.layout_overflow = 0.0;
        }

        Panel::bottom("footer").show(ui, |ui| {
            ui.add_space(4.0);
            self.footer(ui, s);
            ui.add_space(4.0);
        });

        CentralPanel::default().show(ui, |ui| {
            self.header(ui, s);
            let tabs_width = button_width(ui, s.tab_home) + button_width(ui, s.tab_appearance);
            row_or_column(ui, tabs_width, |ui| {
                ui.selectable_value(&mut self.tab, Tab::Home, s.tab_home);
                ui.selectable_value(&mut self.tab, Tab::Appearance, s.tab_appearance);
            });
            ui.separator();
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                match self.tab {
                    Tab::Home => self.home_tab(ui, s),
                    Tab::Appearance => self.appearance_tab(ui, s),
                }
                self.scroll_focus_into_view(ui);
                #[cfg(test)]
                {
                    // Visible area = clip rect (max_rect grows with oversized content).
                    let over = ui.min_rect().right() - ui.clip_rect().right();
                    self.layout_overflow = self.layout_overflow.max(over);
                }
            });
            #[cfg(test)]
            {
                // Visible area = clip rect (max_rect grows with oversized content).
                let over = ui.min_rect().right() - ui.clip_rect().right();
                self.layout_overflow = self.layout_overflow.max(over);
            }
        });
    }

    /// egui moves keyboard focus with Tab but doesn't scroll to it; do that here so the
    /// focused control is always visible.
    fn scroll_focus_into_view(&mut self, ui: &mut Ui) {
        let focused = ui.ctx().memory(|m| m.focused());
        if focused == self.last_focus {
            return;
        }
        self.last_focus = focused;
        if let Some(response) = focused.and_then(|id| ui.ctx().read_response(id)) {
            ui.scroll_to_rect(response.rect.expand(8.0), None);
        }
    }

    /// Title and status chips. Each chip is also a shortcut for its feature.
    fn header(&mut self, ui: &mut Ui, s: &'static Strings) {
        ui.heading(s.app_title);
        let has_area = self.saved_area.is_some();
        let lupa_open = self.lupa.is_some();
        let passthrough = self.config.lupa_passthrough;
        let area_text = match self.saved_area {
            Some(area) => {
                let (_, _, w, h) = area.to_rect_px();
                (s.chip_area)(w, h)
            }
            None => s.chip_no_area.to_owned(),
        };
        let (lupa_text, lupa_tip) = if lupa_open {
            (s.chip_lupa_open, s.chip_lupa_open_tip)
        } else {
            (s.chip_lupa_closed, s.chip_lupa_closed_tip)
        };
        let passthrough_text = if passthrough {
            s.chip_passthrough_on
        } else {
            s.chip_passthrough_off
        };

        let (mut select_area, mut toggle_lupa, mut toggle_passthrough) = (false, false, false);
        a11y_group(ui, Role::Group, s.status_group, |ui| {
            ui.horizontal_wrapped(|ui| {
                select_area =
                    chip_button(ui, &area_text, has_area, true, s.chip_area_tip).clicked();
                toggle_lupa =
                    chip_button(ui, lupa_text, lupa_open, has_area || lupa_open, lupa_tip)
                        .clicked();
                toggle_passthrough = chip_button(
                    ui,
                    passthrough_text,
                    passthrough,
                    true,
                    s.chip_passthrough_tip,
                )
                .clicked();
            });
        });

        if select_area {
            self.open_selector();
        }
        if toggle_lupa {
            if lupa_open {
                self.close_lupa();
            } else {
                let ctx = ui.ctx().clone();
                self.open_lupa(&ctx);
            }
        }
        if toggle_passthrough {
            self.config.lupa_passthrough = !passthrough;
        }
    }

    fn home_tab(&mut self, ui: &mut Ui, s: &'static Strings) {
        card(ui, s.card_area, |ui| {
            if ui
                .button(s.select_area)
                .on_hover_text(s.select_area_tip)
                .clicked()
            {
                self.open_selector();
            }
            match self.saved_area {
                Some(area) => {
                    let (x, y, w, h) = area.to_rect_px();
                    ui.label((s.area_info)(w, h, x, y));
                }
                None => {
                    ui.weak(s.no_area_hint);
                }
            }
        });

        ui.add_space(8.0);
        card(ui, s.card_lupa, |ui| {
            if self.lupa.is_some() {
                if ui
                    .button(s.close_lupa)
                    .on_hover_text(s.close_lupa_tip)
                    .clicked()
                {
                    self.close_lupa();
                }
            } else {
                let open = ui
                    .add_enabled(self.saved_area.is_some(), Button::new(s.open_lupa))
                    .on_hover_text(s.open_lupa_tip)
                    .on_disabled_hover_text(s.open_lupa_needs_area_tip);
                if open.clicked() {
                    let ctx = ui.ctx().clone();
                    self.open_lupa(&ctx);
                }
            }

            toggle_switch(
                ui,
                &mut self.config.lupa_passthrough,
                s.passthrough,
                s.on,
                s.off,
            )
            .on_hover_text(s.passthrough_tip);

            ui.add_space(4.0);
            self.zoom_controls(ui, s);

            let value = format!("{} fps", self.config.lupa_fps);
            let fps = Slider::new(&mut self.config.lupa_fps, LUPA_FPS_RANGE).suffix(" fps");
            slider_with_caption(ui, s.fps, &value, "60 fps", fps).on_hover_text(s.fps_tip);

            ui.add_space(4.0);
            self.position_controls(ui, s);
        });

        ui.add_space(4.0);
        ui.weak(s.autosave_note);
    }

    fn zoom_controls(&mut self, ui: &mut Ui, s: &'static Strings) {
        let (min, max) = (*LUPA_ZOOM_RANGE.start(), *LUPA_ZOOM_RANGE.end());
        let zoom = &mut self.config.lupa_zoom;
        let sample = "5.00×";
        let button = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
        fn slider(zoom: &mut f32) -> Slider<'_> {
            Slider::new(zoom, LUPA_ZOOM_RANGE)
                .step_by(f64::from(ZOOM_STEP))
                .fixed_decimals(2)
                .suffix("×")
        }
        let minus = |ui: &mut Ui, zoom: &mut f32| {
            if icon_button(ui, *zoom > min, "−", s.zoom_out_tip).clicked() {
                *zoom = (*zoom - ZOOM_STEP).max(min);
            }
        };
        let plus = |ui: &mut Ui, zoom: &mut f32| {
            if icon_button(ui, *zoom < max, "+", s.zoom_in_tip).clicked() {
                *zoom = (*zoom + ZOOM_STEP).min(max);
            }
        };

        // Inline [−] slider [+] when there is room for a usable slider; otherwise the
        // slider gets its own full-width row and the buttons go below it.
        if slider_value_fits(ui, 2.0 * button + 80.0, sample) {
            let label = caption(ui, s.zoom);
            ui.horizontal(|ui| {
                minus(ui, zoom);
                fit_slider_width(ui, button, sample);
                ui.add(slider(zoom)).labelled_by(label.id);
                plus(ui, zoom);
            });
        } else {
            let value = format!("{:.2}×", *zoom);
            slider_with_caption(ui, s.zoom, &value, sample, slider(zoom));
            ui.horizontal(|ui| {
                minus(ui, zoom);
                plus(ui, zoom);
            });
        }
    }

    fn position_controls(&mut self, ui: &mut Ui, s: &'static Strings) {
        let ctx = ui.ctx().clone();
        let screen = ui_selector::screen_size(&ctx);
        caption(ui, s.position);

        // A 3×3 positioning pad: compact at any text size, and each button is named.
        let pad = [
            [
                Some((Placement::TopLeft, "↖", s.preset_top_left)),
                None,
                Some((Placement::TopRight, "↗", s.preset_top_right)),
            ],
            [None, Some((Placement::Center, "◎", s.preset_center)), None],
            [
                Some((Placement::BottomLeft, "↙", s.preset_bottom_left)),
                None,
                Some((Placement::BottomRight, "↘", s.preset_bottom_right)),
            ],
        ];
        let enabled = self.saved_area.is_some();
        // Equal square cells at exact positions; buttons shrink (down to 24 pt) so the pad
        // always fits the width.
        let gap = ui.spacing().item_spacing.x * 0.5;
        let size = ((ui.available_width() - 2.0 * gap) / 3.0)
            .min(ui.spacing().interact_size.y * 1.3)
            .max(24.0);
        let cells = pad.map(|row| row.map(|cell| cell.map(|(_, icon, name)| (icon, name))));
        let clicked = a11y_group(ui, Role::Group, s.position, |ui| {
            icon_pad(ui, &cells, enabled, size, gap)
        });
        if let Some((row, col)) = clicked {
            if let Some((placement, _, _)) = pad[row][col] {
                self.place_lupa(&ctx, placement, screen);
            }
        }

        CollapsingHeader::new(s.advanced)
            .id_salt("advanced_position")
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let x = ui.label(s.pos_x);
                    ui.add(
                        DragValue::new(&mut self.config.lupa_pos_x)
                            .range(0.0..=screen.x)
                            .speed(1.0),
                    )
                    .labelled_by(x.id);
                    let y = ui.label(s.pos_y);
                    ui.add(
                        DragValue::new(&mut self.config.lupa_pos_y)
                            .range(0.0..=screen.y)
                            .speed(1.0),
                    )
                    .labelled_by(y.id);
                });
            });
    }

    fn place_lupa(&mut self, ctx: &Context, placement: Placement, screen: Vec2) {
        let Some(area) = self.saved_area else {
            return;
        };
        let size = lupa::display_size(
            area.to_rect_px(),
            self.config.lupa_zoom,
            ctx.pixels_per_point(),
        );
        let [x, y] = placement.position(screen, size);
        self.config.lupa_pos_x = x;
        self.config.lupa_pos_y = y;
    }

    fn appearance_tab(&mut self, ui: &mut Ui, s: &'static Strings) {
        card(ui, s.card_panel_look, |ui| {
            let appearance = &mut self.config.appearance;

            a11y_group(ui, Role::RadioGroup, s.theme, |ui| {
                caption(ui, s.theme);
                let needed = radio_width(ui, s.theme_system)
                    + radio_width(ui, s.theme_light)
                    + radio_width(ui, s.theme_dark);
                row_or_column(ui, needed, |ui| {
                    ui.radio_value(&mut appearance.theme, ThemeMode::System, s.theme_system);
                    ui.radio_value(&mut appearance.theme, ThemeMode::Light, s.theme_light);
                    ui.radio_value(&mut appearance.theme, ThemeMode::Dark, s.theme_dark);
                });
            });

            toggle_switch(
                ui,
                &mut appearance.high_contrast,
                s.high_contrast,
                s.on,
                s.off,
            )
            .on_hover_text(s.high_contrast_tip);

            // Step buttons instead of a slider: resizing the text while dragging a
            // slider would move the slider under the mouse.
            a11y_group(ui, Role::Group, s.ui_scale, |ui| {
                caption(ui, s.ui_scale).on_hover_text(s.ui_scale_tip);
                let (min, max) = (*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
                let button = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
                let needed = 2.0 * button + text_width(ui, "200%", TextStyle::Body);
                row_or_column(ui, needed, |ui| {
                    let scale = &mut appearance.ui_scale;
                    if icon_button(ui, *scale > min + 0.001, "−", s.ui_scale_decrease).clicked() {
                        *scale = ((*scale - UI_SCALE_STEP).max(min) * 10.0).round() / 10.0;
                    }
                    ui.label(RichText::new(format!("{:.0}%", *scale * 100.0)).strong());
                    if icon_button(ui, *scale < max - 0.001, "+", s.ui_scale_increase).clicked() {
                        *scale = ((*scale + UI_SCALE_STEP).min(max) * 10.0).round() / 10.0;
                    }
                });
            });

            let label = caption(ui, s.language);
            let selected = match self.config.language {
                Some(lang) => lang.native_name(),
                None => s.language_auto,
            };
            ComboBox::from_id_salt("language")
                .width(ui.available_width().min(ui.spacing().combo_width))
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.language, None, s.language_auto);
                    for lang in Lang::ALL {
                        ui.selectable_value(
                            &mut self.config.language,
                            Some(lang),
                            lang.native_name(),
                        );
                    }
                })
                .response
                .labelled_by(label.id);
        });

        ui.add_space(8.0);
        card(ui, s.card_lupa_look, |ui| {
            let appearance = &mut self.config.appearance;
            a11y_group(ui, Role::RadioGroup, s.border_colors, |ui| {
                caption(ui, s.border_colors);
                for preset in BorderPreset::ALL {
                    let name = match preset {
                        BorderPreset::GreenGrey => s.border_green_grey,
                        BorderPreset::BlueOrange => s.border_blue_orange,
                        BorderPreset::YellowMagenta => s.border_yellow_magenta,
                    };
                    ui.radio_value(&mut appearance.border_preset, preset, name);
                }
            });
            let (interactive, passthrough) = appearance.border_preset.colors();
            let needed =
                swatch_width(ui, s.border_interactive) + swatch_width(ui, s.border_passthrough);
            row_or_column(ui, needed, |ui| {
                swatch(ui, interactive, s.border_interactive);
                swatch(ui, passthrough, s.border_passthrough);
            });

            let value = format!("{:.0} px", appearance.border_width);
            let width = Slider::new(&mut appearance.border_width, BORDER_WIDTH_RANGE)
                .step_by(1.0)
                .suffix(" px");
            slider_with_caption(ui, s.border_width, &value, "10 px", width);
        });
    }

    fn footer(&mut self, ui: &mut Ui, s: &'static Strings) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(s.quit).on_hover_text(s.quit_tip).clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
            self.status_ui(ui);
        });
    }

    fn status_ui(&mut self, ui: &mut Ui) {
        let Some(status) = &self.status else {
            return;
        };
        let now = Instant::now();
        if now >= status.until {
            self.status = None;
            return;
        }
        let color = if status.is_error {
            ui.visuals().error_fg_color
        } else {
            theme::current_palette(ui.ctx(), &self.config.appearance).success
        };
        ui.label(RichText::new(&status.text).color(color).strong());
        ui.ctx().request_repaint_after(status.until - now);
    }
}
