//! Accessible theme for people with low vision: large text, large hit targets,
//! strong contrast and a clearly visible keyboard-focus outline.
//!
//! Applied with [`apply`]; light/dark follows Windows unless the user picks one.

use crate::config::{Appearance, ThemeMode, UI_SCALE_RANGE};
use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Stroke, Style, TextStyle, Theme,
    ThemePreference, Visuals,
};

/// Colours of one theme variant.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub panel: Color32,
    pub extreme: Color32,
    pub faint: Color32,
    pub text: Color32,
    pub weak_text: Color32,
    pub widget: Color32,
    pub widget_border: Color32,
    pub hover: Color32,
    pub hover_border: Color32,
    pub active: Color32,
    /// Outline of the focused / pressed widget.
    pub focus: Color32,
    pub selection: Color32,
    pub selection_text: Color32,
    pub error: Color32,
    pub warn: Color32,
    pub success: Color32,
    pub border_width: f32,
}

pub fn palette(dark: bool, high_contrast: bool) -> Palette {
    let rgb = Color32::from_rgb;
    match (dark, high_contrast) {
        (true, false) => Palette {
            dark,
            panel: rgb(21, 23, 28),
            extreme: rgb(11, 12, 15),
            faint: rgb(30, 33, 40),
            text: rgb(242, 244, 248),
            weak_text: rgb(184, 191, 204),
            widget: rgb(42, 47, 58),
            widget_border: rgb(110, 122, 142),
            hover: rgb(52, 59, 73),
            hover_border: rgb(160, 178, 205),
            active: rgb(58, 67, 84),
            focus: rgb(77, 163, 255),
            selection: rgb(30, 102, 208),
            selection_text: Color32::WHITE,
            error: rgb(255, 138, 128),
            warn: rgb(255, 209, 102),
            success: rgb(110, 231, 160),
            border_width: 1.5,
        },
        (false, false) => Palette {
            dark,
            panel: rgb(247, 248, 250),
            extreme: Color32::WHITE,
            faint: rgb(236, 239, 243),
            text: rgb(17, 20, 24),
            weak_text: rgb(74, 83, 97),
            widget: rgb(228, 232, 238),
            widget_border: rgb(122, 132, 148),
            hover: rgb(217, 223, 231),
            hover_border: rgb(63, 74, 90),
            active: rgb(207, 224, 255),
            focus: rgb(11, 87, 208),
            selection: rgb(11, 87, 208),
            selection_text: Color32::WHITE,
            error: rgb(179, 38, 30),
            warn: rgb(138, 90, 0),
            success: rgb(27, 127, 59),
            border_width: 1.5,
        },
        (true, true) => Palette {
            dark,
            panel: Color32::BLACK,
            extreme: Color32::BLACK,
            faint: rgb(20, 20, 20),
            text: Color32::WHITE,
            weak_text: rgb(230, 230, 230),
            widget: Color32::BLACK,
            widget_border: Color32::WHITE,
            hover: rgb(40, 40, 0),
            hover_border: rgb(255, 255, 0),
            active: rgb(0, 40, 40),
            focus: rgb(0, 255, 255),
            selection: rgb(255, 255, 0),
            selection_text: Color32::BLACK,
            error: rgb(255, 120, 120),
            warn: rgb(255, 255, 0),
            success: rgb(0, 255, 127),
            border_width: 2.0,
        },
        (false, true) => Palette {
            dark,
            panel: Color32::WHITE,
            extreme: Color32::WHITE,
            faint: rgb(235, 235, 235),
            text: Color32::BLACK,
            weak_text: rgb(26, 26, 26),
            widget: Color32::WHITE,
            widget_border: Color32::BLACK,
            hover: rgb(232, 232, 255),
            hover_border: rgb(0, 0, 204),
            active: rgb(220, 235, 255),
            focus: rgb(0, 0, 255),
            selection: rgb(0, 0, 128),
            selection_text: Color32::WHITE,
            error: rgb(160, 0, 0),
            warn: rgb(102, 61, 0),
            success: rgb(0, 100, 0),
            border_width: 2.0,
        },
    }
}

/// Applies the theme to every window of the app.
pub fn apply(ctx: &egui::Context, appearance: &Appearance) {
    ctx.set_theme(match appearance.theme {
        ThemeMode::System => ThemePreference::System,
        ThemeMode::Light => ThemePreference::Light,
        ThemeMode::Dark => ThemePreference::Dark,
    });
    for theme in [Theme::Dark, Theme::Light] {
        let palette = palette(theme == Theme::Dark, appearance.high_contrast);
        ctx.set_style_of(theme, style(&palette, appearance.ui_scale));
    }
}

/// The palette currently in use by `ctx` (follows the resolved light/dark theme).
pub fn current_palette(ctx: &egui::Context, appearance: &Appearance) -> Palette {
    palette(ctx.theme() == Theme::Dark, appearance.high_contrast)
}

fn style(p: &Palette, ui_scale: f32) -> Style {
    let s = ui_scale.clamp(*UI_SCALE_RANGE.start(), *UI_SCALE_RANGE.end());
    let mut style = Style {
        visuals: visuals(p),
        // Long button and label texts wrap onto more lines instead of running past the
        // edge of narrow windows.
        wrap_mode: Some(egui::TextWrapMode::Wrap),
        ..Style::default()
    };

    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(15.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(18.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(18.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(26.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(17.0 * s, FontFamily::Monospace),
        ),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(10.0, 10.0) * s;
    spacing.button_padding = egui::vec2(14.0, 8.0) * s;
    spacing.interact_size = egui::vec2(44.0, 36.0) * s;
    spacing.slider_width = 240.0 * s;
    spacing.icon_width = 22.0 * s;
    spacing.icon_width_inner = 12.0 * s;
    spacing.icon_spacing = 8.0 * s;
    spacing.combo_width = 240.0 * s;
    spacing.window_margin = egui::Margin::same((14.0 * s) as i8);
    spacing.menu_margin = egui::Margin::same((10.0 * s) as i8);
    spacing.indent = 22.0 * s;
    style
}

fn visuals(p: &Palette) -> Visuals {
    let mut v = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let radius = CornerRadius::same(6);

    v.override_text_color = Some(p.text);
    v.weak_text_color = Some(p.weak_text);
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.window_stroke = Stroke::new(p.border_width, p.widget_border);
    v.extreme_bg_color = p.extreme;
    v.faint_bg_color = p.faint;
    v.code_bg_color = p.faint;
    v.hyperlink_color = p.focus;
    v.error_fg_color = p.error;
    v.warn_fg_color = p.warn;
    v.selection.bg_fill = p.selection;
    v.selection.stroke = Stroke::new(1.5, p.selection_text);
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.widget_border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = p.widget;
    w.inactive.weak_bg_fill = p.widget;
    w.inactive.bg_stroke = Stroke::new(p.border_width, p.widget_border);
    w.inactive.fg_stroke = Stroke::new(1.5, p.text);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(p.border_width + 0.5, p.hover_border);
    w.hovered.fg_stroke = Stroke::new(2.0, p.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 1.0;

    // egui draws focused widgets with the `active` visuals: a thick outline in the
    // focus colour makes keyboard navigation easy to follow.
    w.active.bg_fill = p.active;
    w.active.weak_bg_fill = p.active;
    w.active.bg_stroke = Stroke::new(3.0, p.focus);
    w.active.fg_stroke = Stroke::new(2.0, p.text);
    w.active.corner_radius = radius;
    w.active.expansion = 1.0;

    w.open = w.active;
    v
}

/// WCAG relative luminance of an sRGB colour.
pub fn relative_luminance(c: Color32) -> f64 {
    let channel = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

/// WCAG contrast ratio between two colours (1.0 – 21.0).
pub fn contrast_ratio(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_contrast(name: &str, fg: Color32, bg: Color32, min: f64) {
        let ratio = contrast_ratio(fg, bg);
        assert!(ratio >= min, "{name}: contrast {ratio:.2} < {min}");
    }

    #[test]
    fn contrast_ratio_reference_values() {
        assert!((contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    /// Normal themes meet WCAG AA (4.5:1) everywhere and AAA (7:1) for body text;
    /// high-contrast themes meet AAA (7:1) everywhere.
    #[test]
    fn all_palettes_meet_wcag_contrast() {
        for dark in [true, false] {
            for high_contrast in [false, true] {
                let p = palette(dark, high_contrast);
                let min = if high_contrast { 7.0 } else { 4.5 };
                let tag = format!("dark={dark} hc={high_contrast}");
                assert_contrast(&format!("{tag} text/panel"), p.text, p.panel, 7.0);
                assert_contrast(&format!("{tag} weak/panel"), p.weak_text, p.panel, min);
                assert_contrast(&format!("{tag} text/widget"), p.text, p.widget, min);
                assert_contrast(&format!("{tag} text/hover"), p.text, p.hover, min);
                assert_contrast(&format!("{tag} text/active"), p.text, p.active, min);
                assert_contrast(
                    &format!("{tag} selection"),
                    p.selection_text,
                    p.selection,
                    min,
                );
                assert_contrast(&format!("{tag} error/panel"), p.error, p.panel, min);
                assert_contrast(&format!("{tag} success/panel"), p.success, p.panel, min);
                // Non-text UI components (focus outline) need 3:1 (WCAG 1.4.11).
                assert_contrast(&format!("{tag} focus/panel"), p.focus, p.panel, 3.0);
            }
        }
    }

    #[test]
    fn ui_scale_enlarges_text_and_targets() {
        let p = palette(true, false);
        let normal = style(&p, 1.0);
        let large = style(&p, 2.0);
        let body = |s: &Style| s.text_styles[&TextStyle::Body].size;
        assert_eq!(body(&normal), 18.0);
        assert_eq!(body(&large), 36.0);
        assert!(large.spacing.interact_size.y > normal.spacing.interact_size.y);
        // Out-of-range scales are clamped.
        assert_eq!(body(&style(&p, 9.0)), 36.0);
    }
}
