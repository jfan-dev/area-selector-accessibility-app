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

impl Strings {
    /// Name of a hotkey action.
    pub fn action_name(&self, action: crate::hotkeys::Action) -> &'static str {
        use crate::hotkeys::Action;
        match action {
            Action::TogglePassthrough => self.action_toggle_passthrough,
            Action::ToggleLupa => self.action_toggle_lupa,
            Action::SelectArea => self.action_select_area,
            Action::ZoomIn => self.action_zoom_in,
            Action::ZoomOut => self.action_zoom_out,
            Action::ShowPanel => self.action_show_panel,
        }
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

    // Hotkeys tab
    pub tab_hotkeys: &'static str,
    pub card_hotkeys: &'static str,
    pub hotkeys_note: &'static str,
    pub action_toggle_passthrough: &'static str,
    pub action_toggle_lupa: &'static str,
    pub action_select_area: &'static str,
    pub action_zoom_in: &'static str,
    pub action_zoom_out: &'static str,
    pub action_show_panel: &'static str,
    pub hotkey_none: &'static str,
    pub hotkey_change: &'static str,
    pub hotkey_clear: &'static str,
    pub hotkey_restore: &'static str,
    pub hotkey_recording: &'static str,
    pub hotkey_err_in_use: &'static str,
    pub hotkey_err_duplicate: &'static str,
    pub hotkey_err_invalid: &'static str,

    // Tray, close-to-tray notice and first-run guide
    pub tray_show: &'static str,
    pub tray_open_lupa: &'static str,
    pub tray_close_lupa: &'static str,
    pub tray_passthrough: &'static str,
    pub tray_select_area: &'static str,
    pub tray_quit: &'static str,
    pub card_window: &'static str,
    pub close_to_tray: &'static str,
    pub close_to_tray_tip: &'static str,
    pub tray_notice_title: &'static str,
    pub tray_notice_text: &'static str,
    pub tray_notice_dont_show: &'static str,
    pub tray_notice_minimize: &'static str,
    pub tray_notice_quit: &'static str,
    pub help_tip: &'static str,
    pub onboarding_title: &'static str,
    pub onboarding_1_title: &'static str,
    pub onboarding_1_text: &'static str,
    pub onboarding_2_title: &'static str,
    pub onboarding_2_text: &'static str,
    pub onboarding_3_title: &'static str,
    pub onboarding_3_text: &'static str,
    pub onboarding_tip: &'static str,
    pub onboarding_skip: &'static str,
    pub onboarding_back: &'static str,
    pub onboarding_next: &'static str,
    pub onboarding_finish: &'static str,
    pub onboarding_step: fn(usize, usize) -> String,

    // Redesign
    pub app_subtitle: &'static str,
    pub card_view: &'static str,
    pub hotkey_needs_modifier: &'static str,

    // Footer and status
    pub quit: &'static str,
    pub quit_tip: &'static str,
    pub saved_settings: &'static str,
    pub saved_area: &'static str,
    pub save_error: fn(&str) -> String,

    // Selector and magnifier windows
    pub selector_hint: &'static str,
    pub selector_too_small: &'static str,
    pub selector_size: fn(u32, u32) -> String,
    pub lupa_loading: &'static str,
    pub lupa_hint: &'static str,
    pub badge_interactive: &'static str,
    pub badge_passthrough: &'static str,
    pub lupa_drag_tip: &'static str,
    pub show_badge: &'static str,
    pub show_badge_tip: &'static str,

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

    tab_hotkeys: "⌨ Atalhos",
    card_hotkeys: "Atalhos globais",
    hotkeys_note: "Funcionam mesmo com o jogo ou outro programa em foco, e com o painel minimizado.",
    action_toggle_passthrough: "Ligar/desligar o passthrough",
    action_toggle_lupa: "Abrir/fechar a lupa",
    action_select_area: "Selecionar área",
    action_zoom_in: "Aumentar o zoom",
    action_zoom_out: "Diminuir o zoom",
    action_show_panel: "Mostrar o painel",
    hotkey_none: "Nenhum",
    hotkey_change: "Alterar",
    hotkey_clear: "🗑 Remover",
    hotkey_restore: "Restaurar padrões",
    hotkey_recording: "Pressione a nova combinação (Ctrl ou Alt + uma tecla). Esc cancela.",
    hotkey_err_in_use: "⚠ Já usado por outro programa",
    hotkey_err_duplicate: "⚠ Usado também em outra ação",
    hotkey_err_invalid: "⚠ Combinação inválida",
    tray_show: "Mostrar painel",
    tray_open_lupa: "Abrir lupa",
    tray_close_lupa: "Fechar lupa",
    tray_passthrough: "Passthrough",
    tray_select_area: "Selecionar área",
    tray_quit: "Sair",
    card_window: "Janela",
    close_to_tray: "Minimizar para a bandeja ao fechar",
    close_to_tray_tip: "Ao fechar o painel, o app continua na bandeja do sistema (perto do relógio) com a lupa e os atalhos funcionando.",
    tray_notice_title: "O app continua na bandeja",
    tray_notice_text: "Ao fechar o painel, o Area Selector continua rodando na bandeja do sistema, perto do relógio. A lupa e os atalhos continuam funcionando. Clique no ícone da bandeja para abrir o painel de novo, ou use \"Sair\" para encerrar.",
    tray_notice_dont_show: "Não mostrar novamente",
    tray_notice_minimize: "Minimizar para a bandeja",
    tray_notice_quit: "Sair de vez",
    help_tip: "Mostrar o guia de introdução",
    onboarding_title: "Bem-vindo(a) ao Area Selector",
    onboarding_1_title: "1. Selecione uma área",
    onboarding_1_text: "Clique em \"Selecionar área\" e arraste sobre a parte da tela que você quer ampliar, por exemplo o minimapa do jogo.",
    onboarding_2_title: "2. Abra e posicione a lupa",
    onboarding_2_text: "Abra a lupa e arraste-a para onde preferir. Use a roda do mouse sobre ela para mudar o zoom.",
    onboarding_3_title: "3. Passthrough e atalhos",
    onboarding_3_text: "Ligue o passthrough para clicar no jogo através da lupa. Os atalhos globais (aba Atalhos) controlam a lupa sem sair do jogo: Ctrl+Alt+P liga e desliga o passthrough.",
    onboarding_tip: "Dica: jogue em modo janela ou tela cheia sem bordas. Na tela cheia exclusiva o Windows esconde a lupa.",
    onboarding_skip: "Pular",
    onboarding_back: "Voltar",
    onboarding_next: "Próximo",
    onboarding_finish: "Começar",
    onboarding_step: |n, total| format!("Passo {n} de {total}"),
    app_subtitle: "Lupa de tela e captura de área",
    card_view: "Zoom e atualização",
    hotkey_needs_modifier: "⚠ Use Ctrl ou Alt junto com uma tecla. Esc cancela.",
    quit: "Sair",
    quit_tip: "Fecha o aplicativo e a lupa.",
    saved_settings: "✔ Configurações salvas",
    saved_area: "✔ Área salva",
    save_error: |err| format!("⚠ Erro ao salvar as configurações: {err}"),

    selector_hint: "Clique e arraste para selecionar uma área. ESC ou botão direito cancela.",
    selector_too_small: "Área muito pequena: arraste um retângulo maior.",
    selector_size: |w, h| format!("{w} × {h} px"),
    lupa_loading: "⌛ Carregando...",
    lupa_hint: "Arraste para mover · Roda do mouse ou +/−: zoom · Setas: mover · P: passthrough · Esc: fechar",
    badge_interactive: "Interativa",
    badge_passthrough: "Passthrough",
    lupa_drag_tip: "Dica: arraste a lupa para movê-la e use a roda do mouse sobre ela para o zoom.",
    show_badge: "Indicador de modo",
    show_badge_tip: "Mostra no canto da lupa se ela está interativa ou em passthrough, além da cor da borda.",

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

    tab_hotkeys: "⌨ Hotkeys",
    card_hotkeys: "Global hotkeys",
    hotkeys_note: "They work even while a game or another program has focus, and with the panel minimized.",
    action_toggle_passthrough: "Turn passthrough on/off",
    action_toggle_lupa: "Open/close the magnifier",
    action_select_area: "Select area",
    action_zoom_in: "Zoom in",
    action_zoom_out: "Zoom out",
    action_show_panel: "Show the panel",
    hotkey_none: "None",
    hotkey_change: "Change",
    hotkey_clear: "🗑 Remove",
    hotkey_restore: "Restore defaults",
    hotkey_recording: "Press the new combination (Ctrl or Alt + a key). Esc cancels.",
    hotkey_err_in_use: "⚠ Already used by another program",
    hotkey_err_duplicate: "⚠ Also used by another action",
    hotkey_err_invalid: "⚠ Invalid combination",
    tray_show: "Show panel",
    tray_open_lupa: "Open magnifier",
    tray_close_lupa: "Close magnifier",
    tray_passthrough: "Passthrough",
    tray_select_area: "Select area",
    tray_quit: "Quit",
    card_window: "Window",
    close_to_tray: "Minimize to tray when closing",
    close_to_tray_tip: "When you close the panel, the app keeps running in the system tray (near the clock) with the magnifier and hotkeys working.",
    tray_notice_title: "The app keeps running in the tray",
    tray_notice_text: "When you close the panel, Area Selector keeps running in the system tray, near the clock. The magnifier and hotkeys keep working. Click the tray icon to open the panel again, or use \"Quit\" to exit.",
    tray_notice_dont_show: "Don't show again",
    tray_notice_minimize: "Minimize to tray",
    tray_notice_quit: "Quit completely",
    help_tip: "Show the introduction guide",
    onboarding_title: "Welcome to Area Selector",
    onboarding_1_title: "1. Select an area",
    onboarding_1_text: "Click \"Select area\" and drag over the part of the screen you want to magnify, for example the game's minimap.",
    onboarding_2_title: "2. Open and place the magnifier",
    onboarding_2_text: "Open the magnifier and drag it wherever you like. Use the mouse wheel over it to change the zoom.",
    onboarding_3_title: "3. Passthrough and hotkeys",
    onboarding_3_text: "Turn on passthrough to click the game through the magnifier. Global hotkeys (Hotkeys tab) control the magnifier without leaving the game: Ctrl+Alt+P turns passthrough on and off.",
    onboarding_tip: "Tip: play in windowed or borderless fullscreen mode. In exclusive fullscreen, Windows hides the magnifier.",
    onboarding_skip: "Skip",
    onboarding_back: "Back",
    onboarding_next: "Next",
    onboarding_finish: "Get started",
    onboarding_step: |n, total| format!("Step {n} of {total}"),
    app_subtitle: "Screen magnifier & area capture",
    card_view: "Zoom & refresh",
    hotkey_needs_modifier: "⚠ Use Ctrl or Alt together with a key. Esc cancels.",
    quit: "Quit",
    quit_tip: "Closes the app and the magnifier.",
    saved_settings: "✔ Settings saved",
    saved_area: "✔ Area saved",
    save_error: |err| format!("⚠ Could not save the settings: {err}"),

    selector_hint: "Click and drag to select an area. ESC or right-click cancels.",
    selector_too_small: "Area too small: drag a larger rectangle.",
    selector_size: |w, h| format!("{w} × {h} px"),
    lupa_loading: "⌛ Loading...",
    lupa_hint: "Drag to move · Mouse wheel or +/−: zoom · Arrows: move · P: passthrough · Esc: close",
    badge_interactive: "Interactive",
    badge_passthrough: "Passthrough",
    lupa_drag_tip: "Tip: drag the magnifier to move it, and use the mouse wheel over it to zoom.",
    show_badge: "Mode indicator",
    show_badge_tip: "Shows in a corner of the magnifier whether it is interactive or in passthrough, besides the border colour.",

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
            s.tab_hotkeys,
            s.card_hotkeys,
            s.hotkeys_note,
            s.action_toggle_passthrough,
            s.action_toggle_lupa,
            s.action_select_area,
            s.action_zoom_in,
            s.action_zoom_out,
            s.action_show_panel,
            s.hotkey_none,
            s.hotkey_change,
            s.hotkey_clear,
            s.hotkey_restore,
            s.hotkey_recording,
            s.hotkey_err_in_use,
            s.hotkey_err_duplicate,
            s.hotkey_err_invalid,
            s.tray_show,
            s.tray_open_lupa,
            s.tray_close_lupa,
            s.tray_passthrough,
            s.tray_select_area,
            s.tray_quit,
            s.card_window,
            s.close_to_tray,
            s.close_to_tray_tip,
            s.tray_notice_title,
            s.tray_notice_text,
            s.tray_notice_dont_show,
            s.tray_notice_minimize,
            s.tray_notice_quit,
            s.help_tip,
            s.onboarding_title,
            s.onboarding_1_title,
            s.onboarding_1_text,
            s.onboarding_2_title,
            s.onboarding_2_text,
            s.onboarding_3_title,
            s.onboarding_3_text,
            s.onboarding_tip,
            s.onboarding_skip,
            s.onboarding_back,
            s.onboarding_next,
            s.onboarding_finish,
            s.app_subtitle,
            s.card_view,
            s.hotkey_needs_modifier,
            s.selector_hint,
            s.selector_too_small,
            s.lupa_hint,
            s.badge_interactive,
            s.badge_passthrough,
            s.lupa_drag_tip,
            s.show_badge,
            s.show_badge_tip,
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
        let panel_symbols = "↖↗◎↙↘−+×%ℹ…";
        for lang in Lang::ALL {
            let s = tr(lang);
            let mut texts: Vec<String> = static_strings(s).iter().map(|t| t.to_string()).collect();
            texts.push((s.chip_area)(300, 200));
            texts.push((s.selector_size)(300, 200));
            texts.push((s.onboarding_step)(1, 3));
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
