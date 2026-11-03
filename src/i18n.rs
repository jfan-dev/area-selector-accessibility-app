//! Interface translations (Português do Brasil / English).
//!
//! Every visible string is a field of [`Strings`], so a translation missing from one
//! language is a compile error. Messages with values are plain functions.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Pt,
    En,
}

impl Lang {
    pub const ALL: [Self; 2] = [Self::Pt, Self::En];

    /// Language name written in that language (for the language picker).
    pub fn native_name(self) -> &'static str {
        match self {
            Self::Pt => "Português (Brasil)",
            Self::En => "English",
        }
    }

    /// Windows display language: Portuguese if it is any Portuguese variant, else English.
    pub fn system() -> Self {
        // LANGID: the low 10 bits are the primary language; 0x16 = LANG_PORTUGUESE.
        let lang_id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        if lang_id & 0x3ff == 0x16 {
            Self::Pt
        } else {
            Self::En
        }
    }

    /// The configured language, or the Windows one when not set.
    pub fn resolve(configured: Option<Self>) -> Self {
        configured.unwrap_or_else(Self::system)
    }
}

/// Returns the strings for a language.
pub fn tr(lang: Lang) -> &'static Strings {
    match lang {
        Lang::Pt => &PT,
        Lang::En => &EN,
    }
}

pub struct Strings {
    // Panel: header and tabs
    pub app_title: &'static str,
    pub tab_home: &'static str,
    pub tab_appearance: &'static str,
    pub chip_no_area: &'static str,
    pub chip_area: fn(u32, u32) -> String,
    pub chip_lupa_open: &'static str,
    pub chip_lupa_closed: &'static str,
    pub chip_passthrough_on: &'static str,
    pub chip_passthrough_off: &'static str,
    pub chip_area_tip: &'static str,
    pub chip_lupa_open_tip: &'static str,
    pub chip_lupa_closed_tip: &'static str,
    pub chip_passthrough_tip: &'static str,

    // Home: area card
    pub card_area: &'static str,
    pub select_area: &'static str,
    pub select_area_tip: &'static str,
    pub area_info: fn(u32, u32, u32, u32) -> String,
    pub no_area_hint: &'static str,

    // Home: magnifier card
    pub card_lupa: &'static str,
    pub open_lupa: &'static str,
    pub close_lupa: &'static str,
    pub open_lupa_tip: &'static str,
    pub open_lupa_needs_area_tip: &'static str,
    pub close_lupa_tip: &'static str,
    pub passthrough: &'static str,
    pub passthrough_tip: &'static str,
    pub on: &'static str,
    pub off: &'static str,
    pub zoom: &'static str,
    pub zoom_out_tip: &'static str,
    pub zoom_in_tip: &'static str,
    pub fps: &'static str,
    pub fps_tip: &'static str,
    pub position: &'static str,
    pub preset_top_left: &'static str,
    pub preset_top_right: &'static str,
    pub preset_center: &'static str,
    pub preset_bottom_left: &'static str,
    pub preset_bottom_right: &'static str,
    pub advanced: &'static str,
    pub pos_x: &'static str,
    pub pos_y: &'static str,
    pub autosave_note: &'static str,

    // Appearance tab
    pub theme: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub high_contrast: &'static str,
    pub high_contrast_tip: &'static str,
    pub ui_scale: &'static str,
    pub ui_scale_tip: &'static str,
    pub ui_scale_decrease: &'static str,
    pub ui_scale_increase: &'static str,
    pub status_group: &'static str,
    pub language: &'static str,
    pub language_auto: &'static str,
    pub border_colors: &'static str,
    pub border_green_grey: &'static str,
    pub border_blue_orange: &'static str,
    pub border_yellow_magenta: &'static str,
    pub border_width: &'static str,
    pub border_interactive: &'static str,
    pub border_passthrough: &'static str,
    pub card_panel_look: &'static str,
    pub card_lupa_look: &'static str,

    // Footer and status
    pub quit: &'static str,
    pub quit_tip: &'static str,
    pub saved_settings: &'static str,
    pub saved_area: &'static str,
    pub save_error: fn(&str) -> String,

    // Selector and magnifier windows
    pub selector_hint: &'static str,
    pub lupa_loading: &'static str,

    // Command line
    pub cli_usage: &'static str,
    pub cli_error: &'static str,
    pub cli_unknown_mode: fn(&str) -> String,
    pub cli_seletor_no_args: &'static str,
    pub cli_lupa_args: &'static str,
    pub cli_zero_size: &'static str,
    pub cli_invalid_value: fn(&str, &str) -> String,
    pub cli_invalid_zoom: fn(&str) -> String,
    pub cli_area_saved: fn(u32, u32, u32, u32) -> String,
    pub cli_position_saved: fn(f32, f32) -> String,
    pub cli_save_failed: fn(&str) -> String,
}

pub static PT: Strings = Strings {
    app_title: "Area Selector",
    tab_home: "🏠 Início",
    tab_appearance: "🎨 Aparência",
    chip_no_area: "Sem área",
    chip_area: |w, h| format!("Área {w}×{h}"),
    chip_lupa_open: "Lupa aberta",
    chip_lupa_closed: "Lupa fechada",
    chip_passthrough_on: "Passthrough ligado",
    chip_passthrough_off: "Passthrough desligado",
    chip_area_tip: "Selecionar uma nova área",
    chip_lupa_open_tip: "Fechar a lupa",
    chip_lupa_closed_tip: "Abrir a lupa (selecione uma área primeiro)",
    chip_passthrough_tip: "Ligar ou desligar o passthrough",

    card_area: "Área da tela",
    select_area: "📐 Selecionar área",
    select_area_tip:
        "Abre uma captura da tela: clique e arraste sobre a região que deseja ampliar.",
    area_info: |w, h, x, y| format!("{w} × {h} px, a partir de ({x}, {y})"),
    no_area_hint: "Nenhuma área selecionada. Comece selecionando a região que deseja ampliar.",

    card_lupa: "Lupa",
    open_lupa: "🔍 Abrir lupa",
    close_lupa: "✖ Fechar lupa",
    open_lupa_tip: "Abre a janela ampliada da área selecionada.",
    open_lupa_needs_area_tip: "Selecione uma área primeiro.",
    close_lupa_tip: "Fecha a lupa (também funciona no modo passthrough).",
    passthrough: "Passthrough",
    passthrough_tip: "Quando ligado, os cliques atravessam a lupa e chegam ao jogo por baixo dela.",
    on: "ligado",
    off: "desligado",
    zoom: "Zoom",
    zoom_out_tip: "Diminuir o zoom",
    zoom_in_tip: "Aumentar o zoom",
    fps: "Quadros por segundo",
    fps_tip: "Com que frequência a lupa se atualiza. Valores menores usam menos processador.",
    position: "Posição da lupa",
    preset_top_left: "Canto superior esquerdo",
    preset_top_right: "Canto superior direito",
    preset_center: "Centro da tela",
    preset_bottom_left: "Canto inferior esquerdo",
    preset_bottom_right: "Canto inferior direito",
    advanced: "Avançado",
    pos_x: "X",
    pos_y: "Y",
    autosave_note: "As configurações são salvas automaticamente.",

    theme: "Tema",
    theme_system: "Igual ao Windows",
    theme_light: "Claro",
    theme_dark: "Escuro",
    high_contrast: "Alto contraste",
    high_contrast_tip: "Preto e branco puros com destaques amarelos, para máxima legibilidade.",
    ui_scale: "Tamanho do texto",
    ui_scale_tip: "Aumenta o texto e os botões do painel.",
    ui_scale_decrease: "Diminuir o tamanho do texto",
    ui_scale_increase: "Aumentar o tamanho do texto",
    status_group: "Estado",
    language: "Idioma",
    language_auto: "Igual ao Windows",
    border_colors: "Cores da borda",
    border_green_grey: "Verde / cinza",
    border_blue_orange: "Azul / laranja (daltonismo)",
    border_yellow_magenta: "Amarelo / magenta (máxima visibilidade)",
    border_width: "Espessura da borda",
    border_interactive: "Interativa",
    border_passthrough: "Passthrough",
    card_panel_look: "Painel",
    card_lupa_look: "Borda da lupa",

    quit: "Sair",
    quit_tip: "Fecha o aplicativo e a lupa.",
    saved_settings: "✔ Configurações salvas",
    saved_area: "✔ Área salva",
    save_error: |err| format!("⚠ Erro ao salvar as configurações: {err}"),

    selector_hint: "Clique e arraste para selecionar uma área. ESC cancela.",
    lupa_loading: "⌛ Carregando...",

    cli_usage: "\
Uso:
  area_selector                       abre o painel principal
  area_selector seletor               abre só o seletor de área
  area_selector lupa <x> <y> <w> <h> [zoom|passthrough] [pos_x pos_y]
                                      abre só a lupa",
    cli_error: "Erro",
    cli_unknown_mode: |mode| format!("modo desconhecido: '{mode}'"),
    cli_seletor_no_args: "'seletor' não aceita argumentos",
    cli_lupa_args: "'lupa' espera <x> <y> <w> <h> [zoom|passthrough] [pos_x pos_y]",
    cli_zero_size: "largura e altura precisam ser maiores que zero",
    cli_invalid_value: |name, value| format!("valor inválido para {name}: '{value}'"),
    cli_invalid_zoom: |value| format!("zoom inválido: '{value}'"),
    cli_area_saved: |w, h, x, y| format!("✅ Área salva: {w}×{h} px em ({x}, {y})"),
    cli_position_saved: |x, y| format!("💾 Nova posição salva: ({x}, {y})"),
    cli_save_failed: |err| format!("Erro ao salvar: {err}"),
};

pub static EN: Strings = Strings {
    app_title: "Area Selector",
    tab_home: "🏠 Home",
    tab_appearance: "🎨 Appearance",
    chip_no_area: "No area",
    chip_area: |w, h| format!("Area {w}×{h}"),
    chip_lupa_open: "Magnifier open",
    chip_lupa_closed: "Magnifier closed",
    chip_passthrough_on: "Passthrough on",
    chip_passthrough_off: "Passthrough off",
    chip_area_tip: "Select a new area",
    chip_lupa_open_tip: "Close the magnifier",
    chip_lupa_closed_tip: "Open the magnifier (select an area first)",
    chip_passthrough_tip: "Turn passthrough on or off",

    card_area: "Screen area",
    select_area: "📐 Select area",
    select_area_tip:
        "Opens a screenshot of your screen: click and drag over the region to magnify.",
    area_info: |w, h, x, y| format!("{w} × {h} px, starting at ({x}, {y})"),
    no_area_hint: "No area selected yet. Start by selecting the region you want to magnify.",

    card_lupa: "Magnifier",
    open_lupa: "🔍 Open magnifier",
    close_lupa: "✖ Close magnifier",
    open_lupa_tip: "Opens the magnified window of the selected area.",
    open_lupa_needs_area_tip: "Select an area first.",
    close_lupa_tip: "Closes the magnifier (also works in passthrough mode).",
    passthrough: "Passthrough",
    passthrough_tip: "When on, clicks go through the magnifier to the game underneath.",
    on: "on",
    off: "off",
    zoom: "Zoom",
    zoom_out_tip: "Zoom out",
    zoom_in_tip: "Zoom in",
    fps: "Frames per second",
    fps_tip: "How often the magnifier refreshes. Lower values use less CPU.",
    position: "Magnifier position",
    preset_top_left: "Top-left corner",
    preset_top_right: "Top-right corner",
    preset_center: "Centre of the screen",
    preset_bottom_left: "Bottom-left corner",
    preset_bottom_right: "Bottom-right corner",
    advanced: "Advanced",
    pos_x: "X",
    pos_y: "Y",
    autosave_note: "Settings are saved automatically.",

    theme: "Theme",
    theme_system: "Same as Windows",
    theme_light: "Light",
    theme_dark: "Dark",
    high_contrast: "High contrast",
    high_contrast_tip: "Pure black and white with yellow highlights, for maximum legibility.",
    ui_scale: "Text size",
    ui_scale_tip: "Makes the panel's text and buttons larger.",
    ui_scale_decrease: "Decrease text size",
    ui_scale_increase: "Increase text size",
    status_group: "Status",
    language: "Language",
    language_auto: "Same as Windows",
    border_colors: "Border colours",
    border_green_grey: "Green / grey",
    border_blue_orange: "Blue / orange (colour-blind friendly)",
    border_yellow_magenta: "Yellow / magenta (maximum visibility)",
    border_width: "Border thickness",
    border_interactive: "Interactive",
    border_passthrough: "Passthrough",
    card_panel_look: "Panel",
    card_lupa_look: "Magnifier border",

    quit: "Quit",
    quit_tip: "Closes the app and the magnifier.",
    saved_settings: "✔ Settings saved",
    saved_area: "✔ Area saved",
    save_error: |err| format!("⚠ Could not save the settings: {err}"),

    selector_hint: "Click and drag to select an area. ESC cancels.",
    lupa_loading: "⌛ Loading...",

    cli_usage: "\
Usage:
  area_selector                       opens the main panel
  area_selector seletor               opens only the area selector
  area_selector lupa <x> <y> <w> <h> [zoom|passthrough] [pos_x pos_y]
                                      opens only the magnifier",
    cli_error: "Error",
    cli_unknown_mode: |mode| format!("unknown mode: '{mode}'"),
    cli_seletor_no_args: "'seletor' takes no arguments",
    cli_lupa_args: "'lupa' expects <x> <y> <w> <h> [zoom|passthrough] [pos_x pos_y]",
    cli_zero_size: "width and height must be greater than zero",
    cli_invalid_value: |name, value| format!("invalid value for {name}: '{value}'"),
    cli_invalid_zoom: |value| format!("invalid zoom: '{value}'"),
    cli_area_saved: |w, h, x, y| format!("✅ Area saved: {w}×{h} px at ({x}, {y})"),
    cli_position_saved: |x, y| format!("💾 New position saved: ({x}, {y})"),
    cli_save_failed: |err| format!("Could not save: {err}"),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every static string of a language, for completeness checks.
    fn static_strings(s: &Strings) -> Vec<&'static str> {
        vec![
            s.app_title,
            s.tab_home,
            s.tab_appearance,
            s.chip_no_area,
            s.chip_lupa_open,
            s.chip_lupa_closed,
            s.chip_passthrough_on,
            s.chip_passthrough_off,
            s.chip_area_tip,
            s.chip_lupa_open_tip,
            s.chip_lupa_closed_tip,
            s.chip_passthrough_tip,
            s.card_area,
            s.select_area,
            s.select_area_tip,
            s.no_area_hint,
            s.card_lupa,
            s.open_lupa,
            s.close_lupa,
            s.open_lupa_tip,
            s.open_lupa_needs_area_tip,
            s.close_lupa_tip,
            s.passthrough,
            s.passthrough_tip,
            s.on,
            s.off,
            s.zoom,
            s.zoom_out_tip,
            s.zoom_in_tip,
            s.fps,
            s.fps_tip,
            s.position,
            s.preset_top_left,
            s.preset_top_right,
            s.preset_center,
            s.preset_bottom_left,
            s.preset_bottom_right,
            s.advanced,
            s.pos_x,
            s.pos_y,
            s.autosave_note,
            s.theme,
            s.theme_system,
            s.theme_light,
            s.theme_dark,
            s.high_contrast,
            s.high_contrast_tip,
            s.ui_scale,
            s.ui_scale_tip,
            s.ui_scale_decrease,
            s.ui_scale_increase,
            s.status_group,
            s.language,
            s.language_auto,
            s.border_colors,
            s.border_green_grey,
            s.border_blue_orange,
            s.border_yellow_magenta,
            s.border_width,
            s.border_interactive,
            s.border_passthrough,
            s.card_panel_look,
            s.card_lupa_look,
            s.quit,
            s.quit_tip,
            s.saved_settings,
            s.saved_area,
            s.selector_hint,
            s.lupa_loading,
            s.cli_usage,
            s.cli_error,
            s.cli_seletor_no_args,
            s.cli_lupa_args,
            s.cli_zero_size,
        ]
    }

    #[test]
    fn no_empty_strings() {
        for lang in Lang::ALL {
            for text in static_strings(tr(lang)) {
                assert!(!text.trim().is_empty(), "empty string in {lang:?}");
            }
        }
    }

    #[test]
    fn languages_are_actually_translated() {
        let differing = static_strings(&PT)
            .iter()
            .zip(static_strings(&EN))
            .filter(|(pt, en)| **pt != *en)
            .count();
        // Only a few identical words are expected ("Zoom", "X", "Y", "Passthrough", ...).
        assert!(
            differing > 50,
            "only {differing} strings differ between PT and EN"
        );
    }

    #[test]
    fn formatted_messages_include_values() {
        for lang in Lang::ALL {
            let s = tr(lang);
            assert!((s.chip_area)(300, 200).contains("300×200"));
            assert!((s.area_info)(300, 200, 10, 20).contains("(10, 20)"));
            assert!((s.cli_unknown_mode)("lixo").contains("lixo"));
            assert!((s.save_error)("disk full").contains("disk full"));
        }
    }

    /// Every character shown in the UI must exist in the fonts egui actually loads,
    /// or it renders as an empty box (this happened when the emoji fonts became an
    /// opt-in feature in egui 0.37).
    #[test]
    fn every_ui_character_has_a_glyph() {
        use eframe::egui::epaint::text::{FontDefinitions, Fonts, TextOptions};
        use eframe::egui::FontId;

        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let font = FontId::proportional(18.0);
        // Symbols drawn by the panel outside the translation tables.
        let panel_symbols = "↖↗◎↙↘−+×%";
        for lang in Lang::ALL {
            let s = tr(lang);
            let mut texts: Vec<String> = static_strings(s).iter().map(|t| t.to_string()).collect();
            texts.push((s.chip_area)(300, 200));
            texts.push((s.cli_area_saved)(1, 2, 3, 4));
            texts.push((s.cli_position_saved)(1.0, 2.0));
            texts.push(panel_symbols.to_owned());
            for text in texts {
                for c in text.chars().filter(|c| !c.is_whitespace()) {
                    assert!(
                        fonts.has_glyph(&font, c),
                        "{lang:?}: no glyph for {c:?} (U+{:04X}) in {text:?}",
                        c as u32
                    );
                }
            }
        }
    }

    #[test]
    fn resolve_prefers_the_configured_language() {
        assert_eq!(Lang::resolve(Some(Lang::En)), Lang::En);
        assert_eq!(Lang::resolve(Some(Lang::Pt)), Lang::Pt);
    }

    #[test]
    fn lang_serializes_as_short_codes() {
        assert_eq!(serde_json::to_string(&Lang::Pt).unwrap(), r#""pt""#);
        assert_eq!(serde_json::from_str::<Lang>(r#""en""#).unwrap(), Lang::En);
    }
}
