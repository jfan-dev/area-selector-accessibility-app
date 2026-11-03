//! Accessible theme: four colour schemes (Light, Dark, High Contrast Light, High
//! Contrast Dark), consistent spacing/typography, and a clearly visible keyboard focus.
//!
//! The scheme follows the theme preference (System / Light / Dark, where System follows
//! Windows and updates automatically) plus the independent High Contrast toggle — see
//! [`variant`]. Widgets read their colours from [`palette_of`] instead of hardcoding them.

use crate::config::{Appearance, ThemeMode, UI_SCALE_RANGE};
use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Id, Margin, Stroke, Style, TextStyle, Theme,
    ThemePreference, Visuals,
};

/// The four colour schemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
}

impl Variant {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark | Self::HighContrastDark)
    }

    pub fn is_high_contrast(self) -> bool {
        matches!(self, Self::HighContrastLight | Self::HighContrastDark)
    }
}

/// Effective scheme: the preference picks light or dark (System = the Windows setting),
/// and High Contrast switches to the high-contrast version of that base.
pub fn variant(preference: ThemeMode, os_dark: bool, high_contrast: bool) -> Variant {
    let dark = match preference {
        ThemeMode::System => os_dark,
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
    };
    match (dark, high_contrast) {
        (false, false) => Variant::Light,
        (true, false) => Variant::Dark,
        (false, true) => Variant::HighContrastLight,
        (true, true) => Variant::HighContrastDark,
    }
}

/// Semantic colours of one scheme (the tokens of the design spec, plus derived states).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub variant: Variant,
    pub background: Color32,
    pub surface: Color32,
    pub surface2: Color32,
    pub text: Color32,
    pub text2: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub border: Color32,
    pub focus: Color32,
    /// Hover background for controls.
    pub hover: Color32,
    /// Slider rail.
    pub track: Color32,
    pub error: Color32,
    pub warn: Color32,
    pub success: Color32,
    /// Panel/card border thickness (about 2 px in high contrast).
    pub border_width: f32,
    pub corner_radius: u8,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub fn palette(variant: Variant) -> Palette {
    match variant {
        Variant::Light => Palette {
            variant,
            background: hex(0xF5F7FA),
            surface: hex(0xFFFFFF),
            surface2: hex(0xE9EFF7),
            text: hex(0x192536),
            text2: hex(0x475569),
            accent: hex(0x245CC5),
            accent_text: hex(0xFFFFFF),
            border: hex(0xC9D3E0),
            focus: hex(0x1249AF),
            hover: hex(0xDCE6F4),
            track: hex(0xC9D3E0),
            error: hex(0xB3261E),
            warn: hex(0x8A5A00),
            success: hex(0x1B7F3B),
            border_width: 1.0,
            corner_radius: 8,
        },
        Variant::Dark => Palette {
            variant,
            background: hex(0x11151C),
            surface: hex(0x1C2330),
            surface2: hex(0x273244),
            text: hex(0xF2F5FA),
            text2: hex(0xB8C5D6),
            accent: hex(0x82A8FF),
            accent_text: hex(0x101827),
            border: hex(0x526078),
            focus: hex(0xB5C9FF),
            hover: hex(0x324058),
            track: hex(0x526078),
            error: hex(0xFF8A80),
            warn: hex(0xFFD166),
            success: hex(0x6EE7A0),
            border_width: 1.0,
            corner_radius: 8,
        },
        Variant::HighContrastLight => Palette {
            variant,
            background: hex(0xFFFFFF),
            surface: hex(0xFFFFFF),
            surface2: hex(0xFFFFFF),
            text: hex(0x000000),
            text2: hex(0x000000),
            accent: hex(0x0033CC),
            accent_text: hex(0xFFFFFF),
            border: hex(0x000000),
            focus: hex(0x0033CC),
            hover: hex(0xE6ECFF),
            track: hex(0x000000),
            error: hex(0xA00000),
            warn: hex(0x663D00),
            success: hex(0x006400),
            border_width: 2.0,
            corner_radius: 4,
        },
        Variant::HighContrastDark => Palette {
            variant,
            background: hex(0x000000),
            surface: hex(0x000000),
            surface2: hex(0x000000),
            text: hex(0xFFFFFF),
            text2: hex(0xFFFFFF),
            accent: hex(0xFFFF00),
            accent_text: hex(0x000000),
            border: hex(0xFFFFFF),
            focus: hex(0xFFFF00),
            hover: hex(0x2A2A00),
            track: hex(0xFFFFFF),
            error: hex(0xFF7070),
            warn: hex(0xFFFF00),
            success: hex(0x00FF7F),
            border_width: 2.0,
            corner_radius: 4,
        },
    }
}

fn palette_id(theme: Theme) -> Id {
    match theme {
        Theme::Dark => Id::unique("theme_palette_dark"),
        Theme::Light => Id::unique("theme_palette_light"),
    }
}

/// Applies the theme to every window of the app. With the System preference, egui
/// switches between the light and dark styles when Windows changes its setting.
pub fn apply(ctx: &egui::Context, appearance: &Appearance) {
    ctx.set_theme(match appearance.theme {
        ThemeMode::System => ThemePreference::System,
        ThemeMode::Light => ThemePreference::Light,
        ThemeMode::Dark => ThemePreference::Dark,
    });
    for (theme, dark) in [(Theme::Dark, true), (Theme::Light, false)] {
        // egui picks its dark or light style from the preference (System = Windows);
        // each gets the matching scheme, high-contrast or not.
        let palette = palette(variant(ThemeMode::System, dark, appearance.high_contrast));
        ctx.set_style_of(theme, style(&palette, appearance.ui_scale));
        ctx.data_mut(|data| data.insert_temp(palette_id(theme), palette));
    }
}

/// The palette in use by `ctx` right now (follows the resolved light/dark theme).
pub fn palette_of(ctx: &egui::Context) -> Palette {
    let theme = ctx.theme();
    ctx.data(|data| data.get_temp(palette_id(theme)))
        .unwrap_or_else(|| {
            palette(if theme == Theme::Dark {
                Variant::Dark
            } else {
                Variant::Light
            })
        })
}

/// Size of the window title relative to body text.
pub const TITLE_SCALE: f32 = 1.5;

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
            FontId::new(13.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(16.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(16.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(19.0 * s, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(15.0 * s, FontFamily::Monospace),
        ),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(8.0, 8.0) * s;
    spacing.button_padding = egui::vec2(12.0, 6.0) * s;
    spacing.interact_size = egui::vec2(40.0, 30.0) * s;
    spacing.slider_width = 200.0 * s;
    spacing.icon_width = 20.0 * s;
    spacing.icon_width_inner = 11.0 * s;
    spacing.icon_spacing = 7.0 * s;
    spacing.combo_width = 220.0 * s;
    spacing.window_margin = Margin::same((14.0 * s) as i8);
    spacing.menu_margin = Margin::same((8.0 * s) as i8);
    spacing.indent = 18.0 * s;
    style
}

fn visuals(p: &Palette) -> Visuals {
    let mut v = if p.variant.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let hc = p.variant.is_high_contrast();
    let radius = CornerRadius::same(p.corner_radius);
    let outline = p.border_width;

    // No global text override: it would also force the normal text colour onto selected
    // tabs/buttons (white on yellow in High Contrast Dark). Each widget state sets its
    // own text colour below; selected widgets use `selection.stroke` (accent text).
    v.override_text_color = None;
    v.weak_text_color = Some(p.text2);
    v.panel_fill = p.background;
    // Popups, dropdowns, dialogs and tooltips use the surface colour with a clear border.
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(outline, p.border);
    v.window_corner_radius = radius;
    v.menu_corner_radius = radius;
    if hc {
        // Solid backgrounds: no shadows or transparency effects.
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
    }
    v.extreme_bg_color = p.surface;
    v.text_edit_bg_color = Some(p.surface);
    v.faint_bg_color = p.surface2;
    v.code_bg_color = p.surface2;
    v.hyperlink_color = p.accent;
    v.error_fg_color = p.error;
    v.warn_fg_color = p.warn;
    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.5, p.accent_text);
    v.slider_trailing_fill = true;
    // egui marks disabled widgets only by opacity; keep disabled text readable in high
    // contrast (black at 60 % on white is still about 5.7:1).
    v.disabled_alpha = if hc { 0.6 } else { 0.45 };

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(outline, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;

    // Resting controls: secondary surface with a visible outline.
    w.inactive.bg_fill = p.surface2;
    w.inactive.weak_bg_fill = p.surface2;
    w.inactive.bg_stroke = Stroke::new(outline, p.border);
    w.inactive.fg_stroke = Stroke::new(if hc { 2.0 } else { 1.5 }, p.text);
    w.inactive.corner_radius = radius;

    // Hover: tinted background and an accent outline.
    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(outline + 0.5, p.accent);
    w.hovered.fg_stroke = Stroke::new(2.0, p.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 1.0;

    // Focus / pressed (egui uses the `active` visuals for both): a thick outline in the
    // focus colour, so keyboard focus is always recognisable.
    w.active.bg_fill = p.hover;
    w.active.weak_bg_fill = p.hover;
    w.active.bg_stroke = Stroke::new(if hc { 3.0 } else { 2.5 }, p.focus);
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

    const ALL: [Variant; 4] = [
        Variant::Light,
        Variant::Dark,
        Variant::HighContrastLight,
        Variant::HighContrastDark,
    ];

    fn assert_contrast(name: &str, fg: Color32, bg: Color32, min: f64) {
        let ratio = contrast_ratio(fg, bg);
        assert!(ratio >= min, "{name}: contrast {ratio:.2} < {min}");
    }

    #[test]
    fn contrast_ratio_reference_values() {
        assert!((contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn effective_theme_follows_the_spec_table() {
        use ThemeMode::*;
        assert_eq!(variant(System, false, false), Variant::Light);
        assert_eq!(variant(System, true, false), Variant::Dark);
        assert_eq!(variant(Light, true, false), Variant::Light);
        assert_eq!(variant(Dark, false, false), Variant::Dark);
        assert_eq!(variant(System, false, true), Variant::HighContrastLight);
        assert_eq!(variant(System, true, true), Variant::HighContrastDark);
        assert_eq!(variant(Light, true, true), Variant::HighContrastLight);
        assert_eq!(variant(Dark, false, true), Variant::HighContrastDark);
    }

    #[test]
    fn palettes_use_the_spec_tokens() {
        let light = palette(Variant::Light);
        assert_eq!(light.accent, hex(0x245CC5));
        let hc_dark = palette(Variant::HighContrastDark);
        assert_eq!(
            (hc_dark.background, hc_dark.accent),
            (Color32::BLACK, hex(0xFFFF00))
        );
        assert_eq!(palette(Variant::HighContrastLight).focus, hex(0x0033CC));
    }

    /// WCAG: normal text ≥ 4.5:1 on every background it is drawn on; relevant non-text
    /// components (focus outline, accent/selected fills, high-contrast outlines and slider
    /// tracks) ≥ 3:1.
    #[test]
    fn all_palettes_meet_wcag_contrast() {
        for v in ALL {
            let p = palette(v);
            let tag = format!("{v:?}");
            for (bg_name, bg) in [
                ("background", p.background),
                ("surface", p.surface),
                ("surface2", p.surface2),
                ("hover", p.hover),
            ] {
                assert_contrast(&format!("{tag} text/{bg_name}"), p.text, bg, 4.5);
                assert_contrast(&format!("{tag} text2/{bg_name}"), p.text2, bg, 4.5);
            }
            assert_contrast(&format!("{tag} accent text"), p.accent_text, p.accent, 4.5);
            assert_contrast(&format!("{tag} error"), p.error, p.surface, 4.5);
            assert_contrast(&format!("{tag} success"), p.success, p.surface, 4.5);
            assert_contrast(&format!("{tag} warn"), p.warn, p.surface, 4.5);
            assert_contrast(&format!("{tag} focus"), p.focus, p.background, 3.0);
            assert_contrast(&format!("{tag} accent"), p.accent, p.background, 3.0);
            if v.is_high_contrast() {
                assert_contrast(&format!("{tag} border"), p.border, p.background, 3.0);
                assert_contrast(&format!("{tag} track"), p.track, p.background, 3.0);
                assert!(
                    p.border_width >= 2.0,
                    "{tag}: high contrast needs ~2 px borders"
                );
            }
        }
    }

    /// Selected tabs/buttons use accent text on the accent fill, normal ones normal text
    /// on the inactive fill (HC Dark: black on yellow / white on black; HC Light: white on
    /// blue / black on white).
    #[test]
    fn widget_states_use_readable_text_colours() {
        for v in ALL {
            let p = palette(v);
            let visuals = visuals(&p);
            assert_eq!(visuals.override_text_color, None, "{v:?}");
            assert_eq!(visuals.selection.bg_fill, p.accent);
            assert_eq!(visuals.selection.stroke.color, p.accent_text);
            let w = &visuals.widgets;
            for (state, fill, text) in [
                (
                    "inactive",
                    w.inactive.weak_bg_fill,
                    w.inactive.fg_stroke.color,
                ),
                ("hovered", w.hovered.weak_bg_fill, w.hovered.fg_stroke.color),
                ("active", w.active.weak_bg_fill, w.active.fg_stroke.color),
                ("label", p.background, w.noninteractive.fg_stroke.color),
            ] {
                assert_contrast(&format!("{v:?} {state}"), text, fill, 4.5);
            }
        }
        let hc_dark = visuals(&palette(Variant::HighContrastDark));
        assert_eq!(
            hc_dark.selection.bg_fill,
            Color32::from_rgb(0xFF, 0xFF, 0x00)
        );
        assert_eq!(hc_dark.selection.stroke.color, Color32::BLACK);
        let hc_light = visuals(&palette(Variant::HighContrastLight));
        assert_eq!(hc_light.selection.bg_fill, hex(0x0033CC));
        assert_eq!(hc_light.selection.stroke.color, Color32::WHITE);
    }

    #[test]
    fn high_contrast_modes_are_equivalent() {
        let light = palette(Variant::HighContrastLight);
        let dark = palette(Variant::HighContrastDark);
        assert_eq!(light.border_width, dark.border_width);
        assert_eq!(light.corner_radius, dark.corner_radius);
        assert_eq!(light.text, dark.background, "inverted");
    }

    #[test]
    fn ui_scale_enlarges_text_and_targets() {
        let p = palette(Variant::Dark);
        let normal = style(&p, 1.0);
        let large = style(&p, 2.0);
        let body = |s: &Style| s.text_styles[&TextStyle::Body].size;
        assert_eq!(body(&normal), 16.0);
        assert_eq!(body(&large), 32.0);
        assert!(large.spacing.interact_size.y > normal.spacing.interact_size.y);
        assert_eq!(
            body(&style(&p, 9.0)),
            32.0,
            "out-of-range scales are clamped"
        );
    }
}
