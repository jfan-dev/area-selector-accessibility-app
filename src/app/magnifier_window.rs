//! Settings window of one magnifier (a separate OS window, one per magnifier).
//!
//! It edits the magnifier's shared [`MagnifierSettings`] directly; actions that need the
//! app (select area, open/close, delete) are returned as [`MagnifierRequest`]s and handled
//! in `App::logic`. It is a *deferred* viewport, so it keeps working while the panel is
//! minimized or hidden. Layout follows the panel: responsive cards, captions above
//! controls, sliders that move their value into the caption when narrow.

use area_selector::config::{
    BorderPreset, MagnifierSettings, BORDER_WIDTH_RANGE, LUPA_FPS_RANGE, LUPA_ZOOM_RANGE,
};
use area_selector::i18n::Strings;
use area_selector::lupa::{self, Placement};
use area_selector::ui_selector;
use area_selector::widgets::{
    a11y_group, caption, card, fit_slider_width, icon_button, icon_pad, info_box, primary_button,
    remove_button, row_or_column, section_gap, slider_value_fits, slider_with_caption, swatch,
    swatch_width, themed_slider, toggle_switch,
};
use eframe::egui::*;

const ZOOM_STEP: f32 = 0.25;
/// Longest magnifier name.
pub const NAME_MAX_CHARS: usize = 40;

/// Something only the app can do for a magnifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagnifierRequest {
    SelectArea(u32),
    ToggleOpen(u32),
    Delete(u32),
}

/// Window state shared between the app and the window's own render callback.
pub struct WindowState {
    pub strings: &'static Strings,
    /// The magnifier is open (label of the Open/Close button).
    pub lupa_open: bool,
    /// Delete was pressed; the confirmation dialog is open.
    pub confirm_delete: bool,
    /// Move the keyboard focus to Cancel when the dialog opens.
    pub focus_cancel: bool,
    /// The window was closed (X); the app drops it on its next frame.
    pub closed: bool,
    /// How far the content sticks out past the window edge (layout tests).
    #[cfg(test)]
    pub overflow: f32,
}

impl WindowState {
    pub fn new(strings: &'static Strings) -> Self {
        Self {
            strings,
            lupa_open: false,
            confirm_delete: false,
            focus_cancel: false,
            closed: false,
            #[cfg(test)]
            overflow: 0.0,
        }
    }
}

pub fn viewport_id(magnifier_id: u32) -> ViewportId {
    ViewportId::from_hash_of(("magnifier_settings", magnifier_id))
}

pub fn viewport_builder(title: String) -> ViewportBuilder {
    let builder = match area_selector::icon::app_icon() {
        Some(icon) => ViewportBuilder::default().with_icon(icon),
        None => ViewportBuilder::default(),
    };
    builder
        .with_title(title)
        .with_inner_size([480.0, 640.0])
        .with_min_inner_size([320.0, 360.0])
        .with_resizable(true)
}

/// The whole window: padded panel with scrolling cards.
pub fn window_ui(
    ui: &mut Ui,
    settings: &mut MagnifierSettings,
    state: &mut WindowState,
) -> Vec<MagnifierRequest> {
    let mut requests = Vec::new();
    let padding = TextStyle::Body.resolve(ui.style()).size as i8;
    let frame = Frame::central_panel(ui.style()).inner_margin(Margin::same(padding));
    CentralPanel::default().frame(frame).show(ui, |ui| {
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            contents(ui, settings, state, &mut requests);
            #[cfg(test)]
            {
                let over = ui.min_rect().right() - ui.clip_rect().right();
                state.overflow = state.overflow.max(over);
            }
        });
    });
    delete_dialog(&ui.ctx().clone(), settings, state, &mut requests);
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        state.closed = true;
    }
    requests
}

fn contents(
    ui: &mut Ui,
    settings: &mut MagnifierSettings,
    state: &mut WindowState,
    requests: &mut Vec<MagnifierRequest>,
) {
    let s = state.strings;
    let id = settings.id;
    let gap = section_gap(ui);

    card(ui, s.card_name, |ui| {
        let width = ui.available_width();
        let label = caption(ui, s.card_name);
        ui.add(
            TextEdit::singleline(&mut settings.name)
                .char_limit(NAME_MAX_CHARS)
                .desired_width(width),
        )
        .labelled_by(label.id);
    });

    ui.add_space(gap);
    card(ui, s.card_area, |ui| {
        match settings.area {
            Some(area) => {
                let (x, y, w, h) = area.to_rect_px();
                ui.label((s.area_info)(w, h, x, y));
            }
            None => {
                ui.weak(s.no_area_hint);
            }
        }
        if primary_button(ui, true, s.select_area)
            .on_hover_text(s.select_area_tip)
            .clicked()
        {
            requests.push(MagnifierRequest::SelectArea(id));
        }
    });

    ui.add_space(gap);
    card(ui, s.card_lupa, |ui| {
        let (label, tip) = if state.lupa_open {
            (s.close_lupa, s.close_lupa_tip)
        } else {
            (s.open_lupa, s.open_lupa_tip)
        };
        let enabled = state.lupa_open || settings.area.is_some();
        if primary_button(ui, enabled, label)
            .on_hover_text(tip)
            .on_disabled_hover_text(s.open_lupa_needs_area_tip)
            .clicked()
        {
            requests.push(MagnifierRequest::ToggleOpen(id));
        }
        toggle_switch(ui, &mut settings.passthrough, s.passthrough, s.on, s.off)
            .on_hover_text(s.passthrough_tip);
    });

    ui.add_space(gap);
    card(ui, s.card_view, |ui| {
        zoom_controls(ui, &mut settings.zoom, s);
        ui.add_space(ui.spacing().item_spacing.y * 0.5);
        let value = format!("{} fps", settings.fps);
        let fps = Slider::new(&mut settings.fps, LUPA_FPS_RANGE).suffix(" fps");
        slider_with_caption(ui, s.fps, &value, "60 fps", fps).on_hover_text(s.fps_tip);
    });

    ui.add_space(gap);
    card(ui, s.position, |ui| {
        position_pad(ui, settings, s);
        advanced_position(ui, settings, s);
    });

    ui.add_space(gap);
    // Border: this magnifier's own colour presets, independent of the panel theme.
    card(ui, s.card_lupa_look, |ui| {
        a11y_group(ui, Role::RadioGroup, s.border_colors, |ui| {
            caption(ui, s.border_colors);
            for preset in BorderPreset::ALL {
                let name = match preset {
                    BorderPreset::GreenGrey => s.border_green_grey,
                    BorderPreset::BlueOrange => s.border_blue_orange,
                    BorderPreset::YellowMagenta => s.border_yellow_magenta,
                };
                ui.radio_value(&mut settings.border_preset, preset, name);
            }
        });
        toggle_switch(ui, &mut settings.show_badge, s.show_badge, s.on, s.off)
            .on_hover_text(s.show_badge_tip);
        let (interactive, passthrough) = settings.border_preset.colors();
        let needed =
            swatch_width(ui, s.border_interactive) + swatch_width(ui, s.border_passthrough);
        row_or_column(ui, needed, |ui| {
            swatch(ui, interactive, s.border_interactive);
            swatch(ui, passthrough, s.border_passthrough);
        });
        let value = format!("{:.0} px", settings.border_width);
        let width = Slider::new(&mut settings.border_width, BORDER_WIDTH_RANGE)
            .step_by(1.0)
            .suffix(" px");
        slider_with_caption(ui, s.border_width, &value, "10 px", width);
    });

    ui.add_space(gap);
    if remove_button(ui, true, s.delete_magnifier).clicked() {
        state.confirm_delete = true;
        state.focus_cancel = true;
    }
}

/// Id of the delete confirmation dialog (layout tests read its rect).
pub fn delete_dialog_id(magnifier_id: u32) -> Id {
    Id::unique(("delete_confirm", magnifier_id))
}

/// "Delete this magnifier?" as a modal dialog over the dimmed window. Cancel has the
/// keyboard focus when it opens, so Enter/Space never delete by accident; Esc or a
/// click outside cancels.
fn delete_dialog(
    ctx: &Context,
    settings: &MagnifierSettings,
    state: &mut WindowState,
    requests: &mut Vec<MagnifierRequest>,
) {
    if !state.confirm_delete {
        return;
    }
    let s = state.strings;
    let (mut delete, mut cancel) = (false, false);
    let response = Modal::new(delete_dialog_id(settings.id)).show(ctx, |ui| {
        ui.set_max_width((ctx.content_rect().width() - 48.0).clamp(200.0, 420.0));
        a11y_group(ui, Role::AlertDialog, s.delete_magnifier, |ui| {
            ui.heading(s.delete_magnifier);
            ui.label((s.delete_confirm)(&settings.name));
            ui.add_space(section_gap(ui) * 0.5);
            ui.horizontal_wrapped(|ui| {
                delete = remove_button(ui, true, s.delete_yes).clicked();
                let no = ui.button(s.delete_no);
                if std::mem::take(&mut state.focus_cancel) {
                    no.request_focus();
                }
                cancel = no.clicked();
            });
        });
    });
    if delete {
        requests.push(MagnifierRequest::Delete(settings.id));
        state.confirm_delete = false;
    } else if cancel || response.should_close() {
        state.confirm_delete = false;
    }
}

fn zoom_controls(ui: &mut Ui, zoom: &mut f32, s: &Strings) {
    let (min, max) = (*LUPA_ZOOM_RANGE.start(), *LUPA_ZOOM_RANGE.end());
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
            themed_slider(ui, slider(zoom)).labelled_by(label.id);
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

/// The five-button positioning pad (corners and centre).
fn position_pad(ui: &mut Ui, settings: &mut MagnifierSettings, s: &Strings) {
    let ctx = ui.ctx().clone();
    let screen = ui_selector::screen_size(&ctx);
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
    let gap = ui.spacing().item_spacing.x * 0.5;
    let size = ((ui.available_width() - 2.0 * gap) / 3.0)
        .min(ui.spacing().interact_size.y * 1.2)
        .max(24.0);
    let cells = pad.map(|row| row.map(|cell| cell.map(|(_, icon, name)| (icon, name))));
    let enabled = settings.area.is_some();
    let clicked = a11y_group(ui, Role::Group, s.position, |ui| {
        icon_pad(ui, &cells, enabled, size, gap)
    });
    if let (Some((row, col)), Some(area)) = (clicked, settings.area) {
        if let Some((placement, _, _)) = pad[row][col] {
            let size = lupa::display_size(area.to_rect_px(), settings.zoom, ctx.pixels_per_point());
            let [x, y] = placement.position(screen, size);
            settings.pos_x = x;
            settings.pos_y = y;
        }
    }
}

/// Collapsed by default: exact X/Y and the drag/zoom tip.
fn advanced_position(ui: &mut Ui, settings: &mut MagnifierSettings, s: &Strings) {
    let screen = ui_selector::screen_size(ui.ctx());
    CollapsingHeader::new(s.advanced)
        .id_salt(("advanced_position", settings.id))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let x = ui.label(s.pos_x);
                ui.add(
                    DragValue::new(&mut settings.pos_x)
                        .range(0.0..=screen.x)
                        .speed(1.0),
                )
                .labelled_by(x.id);
                let y = ui.label(s.pos_y);
                ui.add(
                    DragValue::new(&mut settings.pos_y)
                        .range(0.0..=screen.y)
                        .speed(1.0),
                )
                .labelled_by(y.id);
            });
            info_box(ui, s.lupa_drag_tip);
        });
}
