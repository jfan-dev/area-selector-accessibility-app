//! Small accessible widgets used by the panel. Colours come from the active theme
//! ([`theme::palette_of`]), never hardcoded per widget.

use crate::theme;
use eframe::egui::{
    self, pos2, vec2, Button, Color32, Frame, Label, Margin, Response, RichText, Role, Sense,
    Slider, Stroke, StrokeKind, TextStyle, TextWrapMode, Ui, UiBuilder, WidgetInfo,
};

/// Vertical gap between major sections (about 12 px at 100 % text size).
pub fn section_gap(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y * 1.5
}

/// On/off switch followed by `"{label}: {state}"`. The state is written in words, not
/// only shown by the knob position, and screen readers announce it as a switch.
/// Clicking the text also toggles it; Space/Enter toggle it when focused.
pub fn toggle_switch(
    ui: &mut Ui,
    on: &mut bool,
    label: &str,
    on_text: &str,
    off_text: &str,
) -> Response {
    let height = ui.spacing().interact_size.y;
    let switch_width = height * 1.9;
    let text = format!("{label}: {}", if *on { on_text } else { off_text });
    let needed =
        switch_width + ui.spacing().item_spacing.x + text_width(ui, &text, TextStyle::Body);
    row_or_column(ui, needed, |ui| {
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(switch_width, height), Sense::click());

        // Clickable with the mouse but not a separate Tab stop: keyboard users toggle the
        // switch itself, which already announces the label and state.
        let text_response = ui.add(Label::new(RichText::new(&text).strong()).sense(Sense::CLICK));

        if response.clicked() || text_response.clicked() {
            *on = !*on;
            response.mark_changed();
        }
        let enabled = ui.is_enabled();
        let state = *on;
        response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, state, label));

        if ui.is_rect_visible(rect) {
            let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
            let visuals = ui.style().interact_selectable(&response, *on);
            let track = if *on {
                ui.visuals().selection.bg_fill
            } else {
                visuals.bg_fill
            };
            let radius = 0.5 * rect.height();
            ui.painter()
                .rect(rect, radius, track, visuals.bg_stroke, StrokeKind::Inside);
            let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
            let knob = if *on {
                ui.visuals().selection.stroke.color
            } else {
                visuals.fg_stroke.color
            };
            ui.painter()
                .circle_filled(pos2(x, rect.center().y), 0.7 * radius, knob);
        }
        response | text_response
    })
}

/// Puts the contents in a named node of the accessibility tree, so screen readers
/// announce the group (e.g. "Theme") together with the control inside it.
pub fn a11y_group<R>(
    ui: &mut Ui,
    role: Role,
    label: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.scope_builder(
        UiBuilder::new()
            .accessibility_role(role)
            .accessibility_label(label),
        add_contents,
    )
    .inner
}

/// A titled card (surface colour, theme border, about 14 px padding) filling the
/// available width. It is also a named group for screen readers.
pub fn card<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    card_sized(ui, title, 0.0, add_contents).0
}

/// [`card`] that is at least `min_height` tall (outer size). Also returns the height the
/// card needs for its own content, so cards side by side can share the tallest height
/// (see [`equal_height_row`]).
pub fn card_sized<R>(
    ui: &mut Ui,
    title: &str,
    min_height: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (R, f32) {
    let p = theme::palette_of(ui.ctx());
    let margin = (ui.spacing().item_spacing.x * 1.75) as i8;
    let chrome = 2.0 * f32::from(margin) + 2.0 * p.border_width;
    Frame::NONE
        .fill(p.surface)
        .stroke(Stroke::new(p.border_width, p.border))
        .corner_radius(p.corner_radius)
        .inner_margin(Margin::same(margin))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Before the content: egui reserves the minimum height from the cursor.
            ui.set_min_height((min_height - chrome).max(0.0));
            // Same look in one or two columns: controls keep their natural width
            // (`ui.columns` would otherwise stretch them).
            let (result, content_height) = ui
                .with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    a11y_group(ui, Role::Group, title, |ui| {
                        ui.label(RichText::new(title).heading().strong());
                        ui.add_space(ui.spacing().item_spacing.y * 0.25);
                        let result = add_contents(ui);
                        // Measured inside the group: what the content itself needs,
                        // unaffected by the stretched height of the previous frame.
                        (result, ui.min_rect().height())
                    })
                })
                .inner;
            (result, content_height + chrome)
        })
        .inner
}

/// Height shared by the cards of one side-by-side row: the tallest natural height seen
/// in the previous frame. Call [`set_equal_height_row`] with this frame's tallest one.
pub fn equal_height_row(ui: &Ui, id: egui::Id) -> f32 {
    ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0)
}

/// Stores the row height for the next frame (repaints once if it changed, so the
/// cards settle immediately).
pub fn set_equal_height_row(ui: &Ui, id: egui::Id, height: f32) {
    // Only real passes count: egui's sizing / discarded passes lay text out with other
    // widths, which would make the row grow.
    if ui.is_sizing_pass() || ui.ctx().will_discard() {
        return;
    }
    let previous = equal_height_row(ui, id);
    if (previous - height).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(id, height));
        ui.ctx().request_repaint();
    }
}

/// Primary action: filled with the accent colour.
pub fn primary_button(ui: &mut Ui, enabled: bool, text: &str) -> Response {
    let p = theme::palette_of(ui.ctx());
    let button = Button::new(RichText::new(text).color(p.accent_text).strong()).fill(p.accent);
    ui.add_enabled(enabled, button)
}

/// Removal action: outlined in the error colour. Callers include an icon/word in the
/// text (e.g. "🗑 Remove") so it doesn't rely on colour alone.
pub fn remove_button(ui: &mut Ui, enabled: bool, text: &str) -> Response {
    let p = theme::palette_of(ui.ctx());
    let button = Button::new(RichText::new(text).color(p.error))
        .fill(p.surface)
        .stroke(Stroke::new(p.border_width.max(1.5), p.error));
    ui.add_enabled(enabled, button)
}

/// Supplementary information in smaller text on a distinct background.
pub fn info_box(ui: &mut Ui, text: &str) {
    let p = theme::palette_of(ui.ctx());
    let margin = (ui.spacing().item_spacing.x) as i8;
    Frame::NONE
        .fill(p.surface2)
        .stroke(Stroke::new(p.border_width, p.border))
        .corner_radius(p.corner_radius)
        .inner_margin(Margin::same(margin))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(format!("ℹ {text}")).small().color(p.text2));
        });
}

/// Sample text at a given font size, framed like [`info_box`] (text-size preview).
pub fn text_preview(ui: &mut Ui, text: &str, size: f32) -> Response {
    let p = theme::palette_of(ui.ctx());
    let margin = (ui.spacing().item_spacing.x) as i8;
    Frame::NONE
        .fill(p.surface2)
        .stroke(Stroke::new(p.border_width, p.border))
        .corner_radius(p.corner_radius)
        .inner_margin(Margin::same(margin))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).size(size));
        })
        .response
}

/// Read-only field showing a keyboard shortcut. `highlighted` = recording.
pub fn key_field(ui: &mut Ui, text: &str, highlighted: bool, min_width: f32) -> Response {
    let p = theme::palette_of(ui.ctx());
    let stroke = if highlighted {
        Stroke::new(p.border_width + 1.5, p.focus)
    } else {
        Stroke::new(p.border_width, p.border)
    };
    Frame::NONE
        .fill(p.surface)
        .stroke(stroke)
        .corner_radius(p.corner_radius)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_min_width(min_width);
            ui.label(RichText::new(text).monospace().strong())
        })
        .inner
}

/// Adds a slider drawn with the theme's track colour (black/white in high contrast)
/// and an accent-outlined handle. egui paints the rail with the resting widget fill,
/// so it is changed only around the slider, not for checkboxes or buttons.
pub fn themed_slider(ui: &mut Ui, slider: Slider<'_>) -> Response {
    let p = theme::palette_of(ui.ctx());
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_fill = p.track;
        let width = if p.variant.is_high_contrast() {
            2.5
        } else {
            1.5
        };
        widgets.inactive.fg_stroke = Stroke::new(width, p.accent);
        ui.add(slider)
    })
    .inner
}

/// Strong label used as a visible caption for the control below it.
pub fn caption(ui: &mut Ui, text: &str) -> Response {
    ui.label(RichText::new(text).strong())
}

/// Button showing only a symbol (e.g. "−") with a spoken name and a tooltip.
pub fn icon_button(ui: &mut Ui, enabled: bool, icon: &str, name: &str) -> Response {
    let size = ui.spacing().interact_size.y;
    icon_button_sized(ui, enabled, icon, name, size)
}

/// [`icon_button`] with a given square size in points.
pub fn icon_button_sized(
    ui: &mut Ui,
    enabled: bool,
    icon: &str,
    name: &str,
    size: f32,
) -> Response {
    // Small padding: the symbol alone sets the size, so the button stays `size` wide
    // (the theme's large text padding would make it much wider).
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().button_padding = vec2(4.0, 4.0);
            ui.add_enabled(enabled, Button::new(icon).min_size(vec2(size, size)))
        })
        .inner
        .on_hover_text(name);
    set_spoken_name(ui, &response, name);
    response
}

/// Overrides the name screen readers announce for a widget.
pub fn set_spoken_name(ui: &Ui, response: &Response, name: &str) {
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(name.to_owned()));
}

/// Smallest useful slider rail, in points.
pub const MIN_SLIDER_WIDTH: f32 = 40.0;

/// Width of `text` on one line in the given text style.
pub fn text_width(ui: &Ui, text: &str, style: TextStyle) -> f32 {
    let font = style.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
        .size()
        .x
}

/// Width a slider needs besides its rail: the value box showing `sample` (e.g. "5.00×"),
/// the gaps around it and the handle overhanging the rail ends. Checked by the
/// headless layout test in `app.rs`.
pub fn slider_value_width(ui: &Ui, sample: &str) -> f32 {
    let spacing = ui.spacing();
    text_width(ui, sample, TextStyle::Body)
        + 2.0 * spacing.button_padding.x
        + 2.0 * spacing.item_spacing.x
        + spacing.interact_size.y * 0.5
}

/// Makes the next slider fill the row, keeping `reserve` points for widgets beside it.
/// `sample` is the widest value the slider shows, to leave room for its value box.
pub fn fit_slider_width(ui: &mut Ui, reserve: f32, sample: &str) {
    let width = ui.available_width() - reserve - slider_value_width(ui, sample);
    ui.spacing_mut().slider_width = width.max(MIN_SLIDER_WIDTH);
}

/// Does a slider with a visible value box showing `sample` fit, keeping `reserve`
/// points for widgets beside it?
pub fn slider_value_fits(ui: &Ui, reserve: f32, sample: &str) -> bool {
    ui.available_width() - reserve - slider_value_width(ui, sample) >= MIN_SLIDER_WIDTH
}

/// Caption + full-width slider. When the window is too narrow for the slider's value
/// box, the value moves into the caption ("Zoom: 2.00×") and the rail takes the row.
/// The slider is announced with the caption's name.
pub fn slider_with_caption(
    ui: &mut Ui,
    caption_text: &str,
    value_text: &str,
    sample: &str,
    slider: Slider<'_>,
) -> Response {
    let show_value = slider_value_fits(ui, 0.0, sample);
    let label = if show_value {
        caption(ui, caption_text)
    } else {
        caption(ui, &format!("{caption_text}: {value_text}"))
    };
    let rail = if show_value {
        ui.available_width() - slider_value_width(ui, sample)
    } else {
        ui.available_width() - ui.spacing().interact_size.y * 0.5
    };
    ui.spacing_mut().slider_width = rail.max(MIN_SLIDER_WIDTH);
    themed_slider(ui, slider.show_value(show_value)).labelled_by(label.id)
}

/// Width taken by a radio button with this label.
pub fn radio_width(ui: &Ui, label: &str) -> f32 {
    let spacing = ui.spacing();
    spacing.icon_width
        + spacing.icon_spacing
        + text_width(ui, label, TextStyle::Body)
        + spacing.item_spacing.x
        + 4.0
}

/// Width taken by a button / selectable label with this text.
pub fn button_width(ui: &Ui, label: &str) -> f32 {
    let spacing = ui.spacing();
    text_width(ui, label, TextStyle::Button)
        + 2.0 * spacing.button_padding.x
        + spacing.item_spacing.x
}

/// Lays the contents out side by side when `needed` points fit in the available width,
/// otherwise one below another. Keeps every control visible in narrow windows.
pub fn row_or_column<R>(ui: &mut Ui, needed: f32, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    if needed <= ui.available_width() {
        ui.horizontal(add_contents).inner
    } else {
        ui.vertical(add_contents).inner
    }
}

/// Status chip that is also a button, e.g. "Area 300×200" opens the area selector.
/// `active` chips use the selection colours. Being a single widget, it wraps to the next
/// line as a whole in wrapping rows; its text only wraps when it is too long even for a
/// row of its own.
pub fn chip_button(
    ui: &mut Ui,
    text: &str,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    let visuals = ui.visuals();
    let (fill, text_color) = if active {
        (visuals.selection.bg_fill, visuals.selection.stroke.color)
    } else {
        (visuals.widgets.inactive.bg_fill, visuals.text_color())
    };
    let width = text_width(ui, text, TextStyle::Button) + 2.0 * ui.spacing().button_padding.x;
    let wrap = if width > ui.max_rect().width() {
        TextWrapMode::Wrap
    } else {
        TextWrapMode::Extend
    };
    let button = Button::new(RichText::new(text).color(text_color))
        .fill(fill)
        .corner_radius(255)
        .wrap_mode(wrap);
    ui.add_enabled(enabled, button)
        .on_hover_text(tooltip)
        .on_disabled_hover_text(tooltip)
}

/// A 3×3 pad of square symbol buttons placed at exact positions (`None` = empty cell).
/// Each button has a tooltip and a spoken name. Returns the clicked cell `(row, col)`.
pub fn icon_pad(
    ui: &mut Ui,
    cells: &[[Option<(&str, &str)>; 3]; 3],
    enabled: bool,
    size: f32,
    gap: f32,
) -> Option<(usize, usize)> {
    let side = 3.0 * size + 2.0 * gap;
    let rect = egui::Rect::from_min_size(ui.cursor().min, vec2(side, side));
    let mut clicked = None;
    // One child region for the whole pad (one layout allocation, no extra spacing).
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        if !enabled {
            ui.disable();
        }
        // Small padding: the symbol alone sets the size (see `icon_button_sized`).
        ui.spacing_mut().button_padding = vec2(4.0, 4.0);
        ui.allocate_rect(rect, Sense::hover());
        for (row, cells) in cells.iter().enumerate() {
            for (col, cell) in cells.iter().enumerate() {
                let Some((icon, name)) = cell else { continue };
                let min = rect.min + vec2(col as f32, row as f32) * (size + gap);
                let cell_rect = egui::Rect::from_min_size(min, vec2(size, size));
                let response = ui
                    .put(cell_rect, Button::new(*icon).min_size(vec2(size, size)))
                    .on_hover_text(*name);
                set_spoken_name(ui, &response, name);
                if response.clicked() {
                    clicked = Some((row, col));
                }
            }
        }
    });
    clicked
}

/// Width taken by [`swatch`] with this label.
pub fn swatch_width(ui: &Ui, label: &str) -> f32 {
    let size = ui.text_style_height(&TextStyle::Body);
    size * 1.6 + 2.0 * ui.spacing().item_spacing.x + text_width(ui, label, TextStyle::Body)
}

/// Colour swatch with a label (for the border preview), so colour is never the only cue.
pub fn swatch(ui: &mut Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let size = ui.text_style_height(&TextStyle::Body);
        let (rect, _) = ui.allocate_exact_size(vec2(size * 1.6, size), Sense::hover());
        ui.painter().rect(
            rect,
            4,
            color,
            ui.visuals().widgets.inactive.bg_stroke,
            StrokeKind::Inside,
        );
        ui.label(label);
    });
}
