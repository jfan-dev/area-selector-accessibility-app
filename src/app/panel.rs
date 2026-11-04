//! Control panel UI: header (icon, title, subtitle, help, status chips), the Home /
//! Hotkeys / Appearance tabs, dialogs and the footer.
//!
//! Layout: about 16 px outer padding, cards with about 14 px padding and 12 px between
//! sections. On wide windows related cards sit side by side; on narrow ones everything
//! stacks, captions go above their control and sliders move their value into the
//! caption when needed (checked by the headless layout tests in `app.rs`).
//! Accessibility: every card and choice is a named group in the accessibility tree,
//! and symbol-only buttons have spoken names. Colours come from the theme palette.

use super::{lock, App, MagnifierEntry, Tab};
use area_selector::config::{self, HomeView, ThemeMode, UI_SCALE_RANGE};
use area_selector::hotkeys::{self, Action, BindingError};
use area_selector::i18n::{Lang, Strings};
use area_selector::theme::{self, TITLE_SCALE};
use area_selector::widgets::{
    a11y_group, button_width, caption, card, card_sized, chip_button, equal_height_row,
    icon_button, info_box, key_field, primary_button, radio_width, remove_button, row_or_column,
    section_gap, set_equal_height_row, text_preview, text_width, toggle_switch,
};
use eframe::egui::*;
use std::time::Instant;

const UI_SCALE_STEP: f32 = 0.1;

/// Clamps a text size to the allowed range, rounded to whole 10 % steps.
fn step_scale(scale: f32) -> f32 {
    (scale.clamp(*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end()) * 10.0).round() / 10.0
}

/// Body font size of the current style (grows with the text-size setting).
fn body_size(ui: &Ui) -> f32 {
    TextStyle::Body.resolve(ui.style()).size
}

/// Wide enough for two cards side by side (about 600 px at 100 % text size).
fn two_columns_fit(ui: &Ui) -> bool {
    ui.available_width() >= 38.0 * body_size(ui)
}

impl App {
    pub(super) fn panel_ui(&mut self, ui: &mut Ui) {
        let s = self.strings();
        #[cfg(test)]
        {
            self.layout_overflow = 0.0;
        }
        let padding = (body_size(ui)) as i8; // about 16 px
        let frame = Frame::central_panel(ui.style()).inner_margin(Margin::same(padding));

        Panel::bottom("footer")
            .frame(frame.inner_margin(Margin::symmetric(padding, padding / 2)))
            .show(ui, |ui| self.footer(ui, s));

        let ctx = ui.ctx().clone();
        self.tray_notice_ui(&ctx, s);
        self.onboarding_ui(&ctx, s);

        CentralPanel::default().frame(frame).show(ui, |ui| {
            // Short windows (for the current text size) scroll the header with the
            // content, so the content keeps most of the height.
            let header_scrolls = ui.available_height() < 32.0 * body_size(ui);
            if !header_scrolls {
                self.header_and_tabs(ui, s);
            }
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if header_scrolls {
                    self.header_and_tabs(ui, s);
                }
                match self.tab {
                    Tab::Home => self.home_tab(ui, s),
                    Tab::Hotkeys => self.hotkeys_tab(ui, s),
                    Tab::Appearance => self.appearance_tab(ui, s),
                }
                self.keep_scale_control_in_place(ui);
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
                let over = ui.min_rect().right() - ui.clip_rect().right();
                self.layout_overflow = self.layout_overflow.max(over);
            }
        });
    }

    fn header_and_tabs(&mut self, ui: &mut Ui, s: &'static Strings) {
        self.header(ui, s);
        ui.add_space(section_gap(ui) * 0.5);
        // Tab labels lose their icon when the row is too narrow (compact labels first),
        // then wrap onto more rows.
        let full = [s.tab_home, s.tab_hotkeys, s.tab_appearance];
        let compact = full.map(|label| label.split_once(' ').map_or(label, |(_, t)| t));
        let width = |labels: &[&str; 3]| labels.iter().map(|l| button_width(ui, l)).sum::<f32>();
        let labels = if width(&full) <= ui.available_width() {
            full
        } else {
            compact
        };
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Home, labels[0]);
            ui.selectable_value(&mut self.tab, Tab::Hotkeys, labels[1]);
            ui.selectable_value(&mut self.tab, Tab::Appearance, labels[2]);
        });
        if self.tab != Tab::Hotkeys {
            self.recording = None;
        }
        if self.tab != Tab::Appearance {
            // A text size that was only previewed is dropped when leaving the tab.
            self.pending_ui_scale = None;
        }
        ui.separator();
    }

    /// After Apply changes the text size, scrolls so the Apply button is back at the
    /// same screen height (under the mouse) instead of wherever the reflow put it.
    fn keep_scale_control_in_place(&mut self, ui: &mut Ui) {
        let Some(anchor) = &mut self.scale_anchor else {
            return;
        };
        if anchor.frames_left == 0 {
            self.scale_anchor = None;
            return;
        }
        anchor.frames_left -= 1;
        ui.ctx().request_repaint();
        // The new style reaches the layout a frame or two after the click, and a scroll
        // shows in the layout only a frame after the next one: measuring sooner would
        // add the same correction twice.
        if (body_size(ui) - anchor.body_size).abs() < 0.01 || std::mem::take(&mut anchor.settling) {
            anchor.now = None;
            return;
        }
        // egui limits a scroll to the content height it measured last frame (still the
        // old, shorter one), so keep correcting until the button is back in place.
        match anchor.now.take() {
            Some(now) if (anchor.top - now).abs() > 0.5 => {
                // No animation: an animated scroll would still be moving next frame and
                // the correction would be added twice.
                ui.scroll_with_delta_animation(
                    vec2(0.0, anchor.top - now),
                    style::ScrollAnimation::none(),
                );
                anchor.settling = true;
            }
            _ => self.scale_anchor = None,
        }
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

    /// App icon, title, subtitle and help button, then the status chips (which are also
    /// shortcuts for their feature).
    fn header(&mut self, ui: &mut Ui, s: &'static Strings) {
        let title_size = body_size(ui) * TITLE_SCALE;
        let icon_size = title_size * 1.6;
        let help_size = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
        let title_width = text_width(ui, s.app_title, TextStyle::Body) * TITLE_SCALE;
        let subtitle_width = text_width(ui, s.app_subtitle, TextStyle::Small);
        let spacing = ui.spacing().item_spacing.x;
        let text_block = title_width.max(subtitle_width);
        // Narrow windows drop the decorative icon first, then stack.
        let with_icon = icon_size + spacing + text_block + help_size + spacing;
        let show_icon = with_icon <= ui.available_width();
        let needed = if show_icon {
            with_icon
        } else {
            text_block + help_size + spacing
        };

        let icon = self.icon_texture(ui.ctx()).filter(|_| show_icon);
        let mut help = false;
        row_or_column(ui, needed, |ui| {
            if let Some(icon) = icon {
                ui.add(Image::new((icon.id(), vec2(icon_size, icon_size))));
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(s.app_title).size(title_size).strong());
                ui.label(RichText::new(s.app_subtitle).small().weak());
            });
            help = icon_button(ui, true, "?", s.help_tip).clicked();
        });
        if help {
            self.show_onboarding = true;
            self.onboarding_step = 0;
        }

        let total = self.magnifiers.len();
        let open = self.magnifiers.iter().filter(|e| e.lupa.is_some()).count();
        let passthrough = self
            .magnifiers
            .iter()
            .filter(|e| lock(&e.settings).passthrough)
            .count();
        let count_text = (s.chip_count)(total);
        let open_text = (s.chip_open_count)(open);
        let passthrough_text = (s.chip_passthrough_count)(passthrough);
        let can_add = self.can_add_magnifier();
        let add_tip = if can_add {
            self.with_hotkey(s.chip_add_tip, Action::SelectArea)
        } else {
            (s.add_magnifier_limit)(config::MAX_MAGNIFIERS)
        };
        let open_tip = self.with_hotkey(s.chip_open_all_tip, Action::ToggleLupa);
        let passthrough_tip =
            self.with_hotkey(s.chip_passthrough_all_tip, Action::TogglePassthrough);
        let any = total > 0;

        let (mut add, mut toggle_all, mut passthrough_all) = (false, false, false);
        a11y_group(ui, Role::Group, s.status_group, |ui| {
            ui.horizontal_wrapped(|ui| {
                add = chip_button(ui, &count_text, any, can_add, &add_tip).clicked();
                toggle_all = chip_button(ui, &open_text, open > 0, any, &open_tip).clicked();
                passthrough_all = chip_button(
                    ui,
                    &passthrough_text,
                    passthrough > 0,
                    any,
                    &passthrough_tip,
                )
                .clicked();
            });
        });

        if add {
            self.add_magnifier();
        }
        if toggle_all {
            self.toggle_all_magnifiers(ui.ctx());
        }
        if passthrough_all {
            self.toggle_all_passthrough();
        }
    }

    /// The list of magnifiers: compact cards (two per row on wide windows, with equal
    /// heights), each with Open/Close, passthrough, Settings and Select.
    fn home_tab(&mut self, ui: &mut Ui, s: &'static Strings) {
        let gap = section_gap(ui);
        let can_add = self.can_add_magnifier();
        let add_tip = self.with_hotkey(s.add_magnifier_tip, Action::SelectArea);
        let limit_tip = (s.add_magnifier_limit)(config::MAX_MAGNIFIERS);
        let spacing = ui.spacing().item_spacing.x;
        let toggle_width = button_width(ui, s.view_cards) + button_width(ui, s.view_list);
        let needed = text_width(ui, s.magnifiers_title, TextStyle::Body)
            + toggle_width
            + button_width(ui, s.add_magnifier)
            + 2.0 * spacing;
        let mut add = false;
        let mut view = self.config.home_view;
        row_or_column(ui, needed, |ui| {
            ui.label(RichText::new(s.magnifiers_title).strong());
            a11y_group(ui, Role::RadioGroup, s.view_group, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut view, HomeView::Cards, s.view_cards)
                        .on_hover_text(s.view_cards_tip);
                    ui.selectable_value(&mut view, HomeView::List, s.view_list)
                        .on_hover_text(s.view_list_tip);
                });
            });
            add = primary_button(ui, can_add, s.add_magnifier)
                .on_hover_text(add_tip)
                .on_disabled_hover_text(&limit_tip)
                .clicked();
        });
        self.config.home_view = view;
        if add {
            self.add_magnifier();
        }
        if !can_add {
            ui.weak(&limit_tip);
        }
        ui.add_space(gap * 0.5);

        if self.magnifiers.is_empty() {
            info_box(ui, s.no_magnifiers);
            return;
        }

        let ids: Vec<u32> = self.magnifiers.iter().map(MagnifierEntry::id).collect();
        let selected = self.selected_index().map(|i| ids[i]);
        if view == HomeView::List {
            self.magnifier_list(ui, s, &ids, selected);
        } else if two_columns_fit(ui) {
            for (row_index, pair) in ids.chunks(2).enumerate() {
                if row_index > 0 {
                    ui.add_space(gap);
                }
                // Side by side: both cards share the taller card's height.
                let row = Id::unique(("magnifier_row", row_index));
                let height = equal_height_row(ui, row);
                let top = ui.cursor().min.y;
                let tallest = ui.columns(2, |columns| {
                    let mut tallest = 0.0_f32;
                    for (column, &id) in columns.iter_mut().zip(pair) {
                        let h = self.magnifier_card(column, s, id, selected == Some(id), height);
                        tallest = tallest.max(h);
                    }
                    tallest
                });
                set_equal_height_row(ui, row, tallest);
                #[cfg(test)]
                if row_index == 0 && pair.len() == 2 {
                    self.card_row = (
                        ui.cursor().min.y - top - ui.spacing().item_spacing.y,
                        height,
                    );
                }
                let _ = top;
            }
        } else {
            for (i, &id) in ids.iter().enumerate() {
                if i > 0 {
                    ui.add_space(gap);
                }
                self.magnifier_card(ui, s, id, selected == Some(id), 0.0);
            }
        }
    }

    /// The magnifiers as one framed list, one row each: name, area, Open/Close,
    /// Settings, Select and passthrough. On wide windows each row is a single line with
    /// the columns lined up across rows; on narrower ones the controls go below the name
    /// (on one line while they fit, else the switch gets its own line).
    /// The controls come in the same order either way, so Tab moves through them alike.
    fn magnifier_list(
        &mut self,
        ui: &mut Ui,
        s: &'static Strings,
        ids: &[u32],
        selected: Option<u32>,
    ) {
        let rows: Vec<ListRow> = ids
            .iter()
            .filter_map(|&id| {
                let entry = &self.magnifiers[self.index_of(id)?];
                let settings = lock(&entry.settings);
                let area = match settings.area {
                    Some(area) => {
                        let (_, _, w, h) = area.to_rect_px();
                        (s.chip_area)(w, h)
                    }
                    None => s.chip_no_area.to_owned(),
                };
                Some(ListRow {
                    id,
                    name: settings.name.clone(),
                    area,
                    has_area: settings.area.is_some(),
                    open: entry.lupa.is_some(),
                    selected: selected == Some(id),
                })
            })
            .collect();

        let columns = ListColumns::measure(ui, s, &rows);
        let p = theme::palette_of(ui.ctx());
        let mut clicked = None;
        Frame::NONE
            .fill(p.surface)
            .stroke(Stroke::new(p.border_width, p.border))
            .corner_radius(p.corner_radius)
            .inner_margin(Margin::same(p.border_width as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let inner = ui.available_width() - 2.0 * columns.margin;
                let layout = if columns.total <= inner {
                    ListLayout::OneLine
                } else if columns.buttons + columns.switch <= inner {
                    ListLayout::TwoLines
                } else {
                    ListLayout::Stacked
                };
                ui.spacing_mut().item_spacing.y = 0.0;
                for (row_index, row) in rows.iter().enumerate() {
                    if row_index > 0 {
                        ui.add(Separator::default().spacing(0.0));
                    }
                    let fill = if row.selected {
                        ui.visuals().selection.bg_fill.gamma_multiply(0.35)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.push_id(("magnifier_row", row.id), |ui| {
                        Frame::NONE
                            .fill(fill)
                            .inner_margin(Margin::same(columns.margin as i8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.spacing_mut().item_spacing.y = columns.spacing * 0.5;
                                a11y_group(ui, Role::Group, &row.name, |ui| {
                                    if let Some(action) =
                                        self.list_row(ui, s, row, &columns, layout)
                                    {
                                        clicked = Some((row.id, action));
                                    }
                                });
                            });
                    });
                }
            });

        let Some((id, action)) = clicked else {
            return;
        };
        match (action, self.index_of(id)) {
            (RowAction::Toggle, Some(index)) => self.toggle_magnifier(ui.ctx(), index),
            (RowAction::Settings, Some(index)) => self.open_settings_window(index),
            (RowAction::Select, _) => self.select_magnifier(id),
            (_, None) => {}
        }
    }

    /// The contents of one row of [`Self::magnifier_list`].
    fn list_row(
        &mut self,
        ui: &mut Ui,
        s: &'static Strings,
        row: &ListRow,
        columns: &ListColumns,
        layout: ListLayout,
    ) -> Option<RowAction> {
        // Pads the widget just added to `width`, so the next column starts aligned
        // (only where the columns share a line).
        let pad = |ui: &mut Ui, response: &Response, width: f32, aligned: bool| {
            if aligned {
                ui.add_space((width - response.rect.width() - columns.spacing).max(0.0));
            }
        };
        let one_line = layout == ListLayout::OneLine;
        let controls_line = layout != ListLayout::Stacked;
        let name_and_area = |ui: &mut Ui| {
            let r = ui.label(RichText::new(&row.name).strong());
            pad(ui, &r, columns.name, one_line);
            let mut r = ui.weak(&row.area);
            if !row.has_area {
                r = r.on_hover_text(s.no_area_hint);
            }
            pad(ui, &r, columns.area, one_line);
        };
        let mut action = None;
        let open_button = |ui: &mut Ui| {
            let (label, tip) = if row.open {
                (s.close_lupa, s.close_lupa_tip)
            } else {
                (s.open_lupa, s.open_lupa_tip)
            };
            let r = primary_button(ui, row.open || row.has_area, label)
                .on_hover_text(tip)
                .on_disabled_hover_text(s.open_lupa_needs_area_tip);
            pad(ui, &r, columns.open, controls_line);
            r.clicked()
        };
        let select_tip = self.with_hotkey(s.select_tip, Action::CycleSelection);
        let settings_and_select = |ui: &mut Ui| {
            let settings = icon_button(ui, true, "⚙", s.open_settings_tip).clicked();
            let r = if row.selected {
                ui.label(RichText::new(s.selected_label).strong())
            } else {
                ui.button(s.select_label).on_hover_text(&select_tip)
            };
            pad(ui, &r, columns.select, controls_line);
            (settings, !row.selected && r.clicked())
        };
        let mut buttons = |ui: &mut Ui| {
            let toggle = open_button(ui);
            let (settings, select) = ui.horizontal(settings_and_select).inner;
            action = if toggle {
                Some(RowAction::Toggle)
            } else if settings {
                Some(RowAction::Settings)
            } else if select {
                Some(RowAction::Select)
            } else {
                None
            };
        };
        let shared = self
            .index_of(row.id)
            .map(|index| self.magnifiers[index].settings.clone());
        let switch = |ui: &mut Ui| {
            if let Some(shared) = &shared {
                let mut settings = lock(shared);
                toggle_switch(ui, &mut settings.passthrough, s.passthrough, s.on, s.off)
                    .on_hover_text(s.passthrough_tip);
            }
        };
        match layout {
            ListLayout::OneLine => {
                ui.horizontal(|ui| {
                    name_and_area(ui);
                    buttons(ui);
                    switch(ui);
                });
            }
            ListLayout::TwoLines => {
                ui.horizontal_wrapped(name_and_area);
                ui.horizontal(|ui| {
                    buttons(ui);
                    switch(ui);
                });
            }
            ListLayout::Stacked => {
                ui.horizontal_wrapped(name_and_area);
                row_or_column(ui, columns.buttons, buttons);
                switch(ui);
            }
        }
        action
    }

    /// One magnifier in the Home list. Returns the card's natural height.
    fn magnifier_card(
        &mut self,
        ui: &mut Ui,
        s: &'static Strings,
        id: u32,
        selected: bool,
        min_height: f32,
    ) -> f32 {
        let Some(index) = self.index_of(id) else {
            return 0.0;
        };
        let (name, area) = {
            let settings = lock(&self.magnifiers[index].settings);
            (settings.name.clone(), settings.area)
        };
        let open = self.magnifiers[index].lupa.is_some();
        let toggle_tip = if open {
            s.close_lupa_tip
        } else {
            s.open_lupa_tip
        };
        let select_tip = self.with_hotkey(s.select_tip, Action::CycleSelection);
        let shared = self.magnifiers[index].settings.clone();
        let (mut toggle, mut settings_clicked, mut select) = (false, false, false);
        let height = ui
            .push_id(("magnifier_card", id), |ui| {
                card_sized(ui, &name, min_height, |ui| {
                    match area {
                        Some(area) => {
                            let (_, _, w, h) = area.to_rect_px();
                            ui.weak((s.chip_area)(w, h));
                        }
                        None => {
                            ui.weak(s.no_area_hint);
                        }
                    }
                    let label = if open { s.close_lupa } else { s.open_lupa };
                    let needed = button_width(ui, label)
                        + button_width(ui, s.open_settings)
                        + ui.spacing().item_spacing.x;
                    row_or_column(ui, needed, |ui| {
                        toggle = primary_button(ui, open || area.is_some(), label)
                            .on_hover_text(toggle_tip)
                            .on_disabled_hover_text(s.open_lupa_needs_area_tip)
                            .clicked();
                        settings_clicked = ui
                            .button(s.open_settings)
                            .on_hover_text(s.open_settings_tip)
                            .clicked();
                    });
                    {
                        let mut settings = lock(&shared);
                        toggle_switch(ui, &mut settings.passthrough, s.passthrough, s.on, s.off)
                            .on_hover_text(s.passthrough_tip);
                    }
                    if selected {
                        ui.label(RichText::new(s.selected_label).strong());
                    } else {
                        select = ui
                            .button(s.select_label)
                            .on_hover_text(select_tip)
                            .clicked();
                    }
                })
                .1
            })
            .inner;

        if toggle {
            self.toggle_magnifier(ui.ctx(), index);
        }
        if settings_clicked {
            self.open_settings_window(index);
        }
        if select {
            self.select_magnifier(id);
        }
        height
    }

    /// `"{tip} ({hotkey})"` when the action has a hotkey.
    fn with_hotkey(&self, tip: &str, action: Action) -> String {
        match self.hotkey_label(action) {
            Some(hotkey) => format!("{tip} ({hotkey})"),
            None => tip.to_owned(),
        }
    }

    /// Text shown in the shortcut field while recording: the modifiers held right now.
    fn recording_preview(ui: &Ui) -> String {
        let modifiers = ui.ctx().input(|i| i.modifiers);
        let mut parts = Vec::new();
        if modifiers.ctrl {
            parts.push("Ctrl");
        }
        if modifiers.alt {
            parts.push("Alt");
        }
        if modifiers.shift {
            parts.push("Shift");
        }
        parts.push("…");
        parts.join("+")
    }

    fn hotkeys_tab(&mut self, ui: &mut Ui, s: &'static Strings) {
        self.record_hotkey(ui);
        card(ui, s.card_hotkeys, |ui| {
            info_box(ui, s.hotkeys_note);
            ui.add_space(ui.spacing().item_spacing.y * 0.5);

            // Column widths shared by all rows, so they line up.
            let spacing = ui.spacing().item_spacing.x;
            let name_width = Action::ALL
                .iter()
                .map(|&a| text_width(ui, s.action_name(a), TextStyle::Body))
                .fold(0.0, f32::max);
            let key_width = Action::ALL
                .iter()
                .filter_map(|&a| self.hotkey_label(a))
                .chain(["Ctrl+Alt+PageDown".to_owned()])
                .map(|label| text_width(ui, &label, TextStyle::Monospace))
                .fold(0.0, f32::max);
            let controls_width = key_width
                + 20.0
                + button_width(ui, s.hotkey_change)
                + button_width(ui, s.hotkey_clear)
                + 2.0 * spacing;
            let aligned = name_width + spacing + controls_width <= ui.available_width();
            // Stacked rows (name above) still share the field width when it fits.
            let fixed_key_width = controls_width <= ui.available_width();

            for action in Action::ALL {
                let name = s.action_name(action);
                let recording = self.recording == Some(action);
                let shortcut = if recording {
                    Self::recording_preview(ui)
                } else {
                    self.hotkey_label(action)
                        .unwrap_or_else(|| s.hotkey_none.to_owned())
                };
                a11y_group(ui, Role::Group, name, |ui| {
                    let row = |ui: &mut Ui, this: &mut Self| {
                        // Same field width on every row when it fits; otherwise it wraps.
                        let min_width = if fixed_key_width { key_width } else { 0.0 };
                        key_field(ui, &shortcut, recording, min_width);
                        let change = ui.add(Button::new(s.hotkey_change).selected(recording));
                        if change.clicked() {
                            this.recording = if recording { None } else { Some(action) };
                            this.recording_invalid = false;
                        }
                        let has_binding = this.hotkey_label(action).is_some();
                        if remove_button(ui, has_binding, s.hotkey_clear).clicked() {
                            this.config.hotkeys.insert(action, String::new());
                            this.recording = None;
                        }
                    };
                    if aligned {
                        ui.horizontal(|ui| {
                            let height = ui.spacing().interact_size.y;
                            ui.allocate_ui_with_layout(
                                vec2(name_width, height),
                                Layout::left_to_right(Align::Center),
                                |ui| ui.label(name),
                            );
                            row(ui, self);
                        });
                    } else {
                        caption(ui, name);
                        ui.horizontal_wrapped(|ui| row(ui, self));
                    }

                    let error = if recording {
                        if self.recording_invalid {
                            Some(s.hotkey_needs_modifier)
                        } else {
                            None
                        }
                    } else {
                        self.hotkeys
                            .as_ref()
                            .and_then(|h| h.error(action))
                            .map(|error| match error {
                                BindingError::InUse => s.hotkey_err_in_use,
                                BindingError::Duplicate => s.hotkey_err_duplicate,
                                BindingError::Invalid => s.hotkey_err_invalid,
                            })
                    };
                    if recording && !self.recording_invalid {
                        let color = theme::palette_of(ui.ctx()).focus;
                        ui.label(RichText::new(s.hotkey_recording).small().color(color));
                    }
                    if let Some(text) = error {
                        ui.colored_label(ui.visuals().error_fg_color, text);
                    }
                });
                ui.add_space(ui.spacing().item_spacing.y * 0.25);
            }

            ui.separator();
            if ui.button(s.hotkey_restore).clicked() {
                self.config.hotkeys = hotkeys::default_bindings();
                self.recording = None;
            }
        });
    }

    /// While recording, the next Ctrl/Alt + key combination becomes the action's hotkey;
    /// Esc cancels; a key without Ctrl/Alt shows a message. Global hotkeys are suspended
    /// meanwhile (`App::sync_hotkeys`).
    fn record_hotkey(&mut self, ui: &mut Ui) {
        let Some(action) = self.recording else {
            return;
        };
        let events = ui.ctx().input(|i| i.events.clone());
        for event in events {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                if key == Key::Escape {
                    self.recording = None;
                    return;
                }
                match hotkeys::binding_from_egui(modifiers, key) {
                    Some(binding) => {
                        self.config.hotkeys.insert(action, binding);
                        self.recording = None;
                        self.recording_invalid = false;
                        return;
                    }
                    None => self.recording_invalid = true,
                }
            }
        }
    }

    fn appearance_tab(&mut self, ui: &mut Ui, s: &'static Strings) {
        let gap = section_gap(ui);
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

            // Step buttons instead of a slider, and − / + only change a preview: the
            // panel resizes once, on Apply, and then scrolls the control back under the
            // mouse (see `keep_scale_control_in_place`).
            let current = appearance.ui_scale;
            let pending = self.pending_ui_scale.get_or_insert(current);
            a11y_group(ui, Role::Group, s.ui_scale, |ui| {
                caption(ui, s.ui_scale).on_hover_text(s.ui_scale_tip);
                let (min, max) = (*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
                let button = ui.spacing().interact_size.y + ui.spacing().item_spacing.x;
                let needed = 2.0 * button + text_width(ui, "200%", TextStyle::Body);
                row_or_column(ui, needed, |ui| {
                    if icon_button(ui, *pending > min + 0.001, "−", s.ui_scale_decrease).clicked()
                    {
                        *pending = step_scale(*pending - UI_SCALE_STEP);
                    }
                    ui.label(RichText::new(format!("{:.0}%", *pending * 100.0)).strong());
                    if icon_button(ui, *pending < max - 0.001, "+", s.ui_scale_increase).clicked() {
                        *pending = step_scale(*pending + UI_SCALE_STEP);
                    }
                });
                let changed = (*pending - current).abs() > 0.001;
                if changed {
                    let now = format!("{:.0}%", current * 100.0);
                    ui.label(RichText::new(s.ui_scale_current.replace("{}", &now)).small());
                }
                let size = body_size(ui) * *pending / current.max(0.1);
                text_preview(ui, s.ui_scale_preview, size);
                let needed = button_width(ui, s.ui_scale_apply)
                    + button_width(ui, s.ui_scale_reset)
                    + ui.spacing().item_spacing.x;
                row_or_column(ui, needed, |ui| {
                    let apply = primary_button(ui, changed, s.ui_scale_apply);
                    if let Some(anchor) = &mut self.scale_anchor {
                        // Its id changes when the layout reflows, so report the position.
                        anchor.now = Some(apply.rect.top());
                    }
                    if apply.clicked() {
                        appearance.ui_scale = *pending;
                        // Keep focus here so keyboard users stay on the control too.
                        apply.request_focus();
                        self.scale_anchor = Some(super::ScaleAnchor {
                            top: apply.rect.top(),
                            now: None,
                            settling: false,
                            body_size: body_size(ui),
                            frames_left: 8,
                        });
                    }
                    if ui
                        .add_enabled(changed, Button::new(s.ui_scale_reset))
                        .clicked()
                    {
                        *pending = current;
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

        ui.add_space(gap);
        card(ui, s.card_window, |ui| {
            toggle_switch(
                ui,
                &mut self.config.close_to_tray,
                s.close_to_tray,
                s.on,
                s.off,
            )
            .on_hover_text(s.close_to_tray_tip);
        });
    }

    /// Width for dialogs: comfortable on wide windows, never wider than the window.
    fn dialog_width(ctx: &Context) -> f32 {
        (ctx.content_rect().width() - 48.0).clamp(200.0, 520.0)
    }

    /// One-time explanation of close-to-tray, shown on the first close.
    fn tray_notice_ui(&mut self, ctx: &Context, s: &'static Strings) {
        if !self.show_tray_notice {
            return;
        }
        let (mut minimize, mut quit) = (false, false);
        let response = Modal::new(Id::unique("tray_notice")).show(ctx, |ui| {
            ui.set_max_width(Self::dialog_width(ctx));
            ui.heading(s.tray_notice_title);
            ui.label(s.tray_notice_text);
            ui.checkbox(&mut self.tray_notice_dont_show, s.tray_notice_dont_show);
            ui.horizontal_wrapped(|ui| {
                minimize = primary_button(ui, true, s.tray_notice_minimize).clicked();
                quit = ui.button(s.tray_notice_quit).clicked();
            });
        });
        if minimize {
            if self.tray_notice_dont_show {
                self.config.tray_notice_shown = true;
            }
            self.show_tray_notice = false;
            self.hide_to_tray(ctx);
        } else if quit {
            self.show_tray_notice = false;
            self.quit(ctx);
        } else if response.should_close() {
            self.show_tray_notice = false; // Esc / click outside: keep the panel open
        }
    }

    /// First-run guide: three short steps plus the fullscreen tip.
    fn onboarding_ui(&mut self, ctx: &Context, s: &'static Strings) {
        if !self.show_onboarding {
            return;
        }
        let steps = [
            (s.onboarding_1_title, s.onboarding_1_text),
            (s.onboarding_2_title, s.onboarding_2_text),
            (s.onboarding_3_title, s.onboarding_3_text),
        ];
        let step = self.onboarding_step.min(steps.len() - 1);
        let last = step == steps.len() - 1;
        let (mut back, mut next, mut done) = (false, false, false);
        let response = Modal::new(Id::unique("onboarding")).show(ctx, |ui| {
            ui.set_max_width(Self::dialog_width(ctx));
            ui.heading(s.onboarding_title);
            ui.weak((s.onboarding_step)(step + 1, steps.len()));
            ui.add_space(4.0);
            let (title, text) = steps[step];
            caption(ui, title);
            ui.label(text);
            if last {
                ui.add_space(4.0);
                info_box(ui, s.onboarding_tip);
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if last {
                    done = primary_button(ui, true, s.onboarding_finish).clicked();
                } else {
                    next = primary_button(ui, true, s.onboarding_next).clicked();
                }
                if step > 0 {
                    back = ui.button(s.onboarding_back).clicked();
                }
                if !last {
                    done |= ui.button(s.onboarding_skip).clicked();
                }
            });
        });
        if back {
            self.onboarding_step = step - 1;
        } else if next {
            self.onboarding_step = step + 1;
        } else if done || response.should_close() {
            self.show_onboarding = false;
            self.onboarding_step = 0;
            self.config.onboarding_done = true;
        }
    }

    /// Status / auto-save note on the left, Quit on the right.
    fn footer(&mut self, ui: &mut Ui, s: &'static Strings) {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(s.quit).on_hover_text(s.quit_tip).clicked() {
                    let ctx = ui.ctx().clone();
                    self.quit(&ctx);
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if !self.status_ui(ui) {
                        ui.label(RichText::new(s.autosave_note).small().weak());
                    }
                });
            });
        });
    }

    /// Shows the current status message; returns false when there is none.
    fn status_ui(&mut self, ui: &mut Ui) -> bool {
        let Some(status) = &self.status else {
            return false;
        };
        let now = Instant::now();
        if now >= status.until {
            self.status = None;
            return false;
        }
        let p = theme::palette_of(ui.ctx());
        let color = if status.is_error { p.error } else { p.success };
        ui.label(RichText::new(&status.text).color(color).strong());
        ui.ctx().request_repaint_after(status.until - now);
        true
    }
}

/// What one row of the Home list shows.
struct ListRow {
    id: u32,
    name: String,
    /// Area size, or "No area".
    area: String,
    has_area: bool,
    open: bool,
    selected: bool,
}

/// How the rows of the Home list are laid out, from wide windows to narrow ones.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListLayout {
    /// Name, area and every control on one line, in columns.
    OneLine,
    /// Name and area, then the controls on one line.
    TwoLines,
    /// Name and area, the buttons (stacked if needed), then the switch.
    Stacked,
}

/// A button clicked in a row of the Home list.
enum RowAction {
    Toggle,
    Settings,
    Select,
}

/// Column widths of the Home list (each including the spacing after it), shared by
/// every row so the controls line up.
struct ListColumns {
    name: f32,
    area: f32,
    open: f32,
    select: f32,
    /// Open/Close, Settings and Select together.
    buttons: f32,
    /// The passthrough switch with its longest text.
    switch: f32,
    /// Width of a whole row on one line, without the row padding.
    total: f32,
    /// Row padding on each side.
    margin: f32,
    spacing: f32,
}

impl ListColumns {
    fn measure(ui: &Ui, s: &Strings, rows: &[ListRow]) -> Self {
        let spacing = ui.spacing().item_spacing.x;
        let widest = |texts: &mut dyn Iterator<Item = &str>| {
            texts
                .map(|t| text_width(ui, t, TextStyle::Body))
                .fold(0.0, f32::max)
                + spacing
        };
        let name = widest(&mut rows.iter().map(|r| r.name.as_str()));
        let area = widest(&mut rows.iter().map(|r| r.area.as_str()));
        let open = button_width(ui, s.open_lupa).max(button_width(ui, s.close_lupa));
        let settings = ui.spacing().interact_size.y + spacing;
        let select = button_width(ui, s.select_label)
            .max(text_width(ui, s.selected_label, TextStyle::Body) + spacing);
        let switch = ui.spacing().interact_size.y * 1.9
            + spacing
            + [s.on, s.off]
                .map(|state| {
                    text_width(ui, &format!("{}: {state}", s.passthrough), TextStyle::Body)
                })
                .into_iter()
                .fold(0.0, f32::max);
        Self {
            name,
            area,
            open,
            select,
            buttons: open + settings + select,
            switch,
            total: name + area + open + settings + select + switch,
            margin: spacing * 1.25,
            spacing,
        }
    }
}
