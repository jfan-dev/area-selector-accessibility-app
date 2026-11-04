//! System-wide keyboard shortcuts (work while a game or another app has focus).
//!
//! Bindings are stored as strings like `"control+alt+KeyP"` (the `global-hotkey`
//! format). [`HotkeyService`] registers them with Windows and turns key presses into
//! [`Action`]s.

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{self, Receiver};

/// Something a global hotkey can do.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    TogglePassthrough,
    ToggleLupa,
    SelectArea,
    ZoomIn,
    ZoomOut,
    ShowPanel,
    /// Selects the next magnifier (the target of zoom / select area).
    CycleSelection,
}

impl Action {
    pub const ALL: [Self; 7] = [
        Self::TogglePassthrough,
        Self::ToggleLupa,
        Self::SelectArea,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::CycleSelection,
        Self::ShowPanel,
    ];

    /// Default combination. Chosen to avoid known conflicts: Ctrl+Alt+arrows rotate the
    /// screen with many Intel graphics drivers, and Ctrl+Alt is AltGr on Brazilian
    /// (ABNT2) keyboards, where AltGr+=, AltGr+Q, AltGr+W, AltGr+E… type symbols.
    pub fn default_binding(self) -> &'static str {
        match self {
            Self::TogglePassthrough => "control+alt+KeyP",
            Self::ToggleLupa => "control+alt+KeyL",
            Self::SelectArea => "control+alt+KeyA",
            Self::ZoomIn => "control+alt+PageUp",
            Self::ZoomOut => "control+alt+PageDown",
            Self::ShowPanel => "control+alt+KeyH",
            Self::CycleSelection => "control+alt+KeyN",
        }
    }
}

/// Action → binding string. An empty string means "no hotkey".
pub type Bindings = BTreeMap<Action, String>;

pub fn default_bindings() -> Bindings {
    Action::ALL
        .iter()
        .map(|&action| (action, action.default_binding().to_owned()))
        .collect()
}

/// Adds default bindings for actions missing from `bindings` (configs from older
/// versions, or actions added later).
pub fn with_defaults(mut bindings: Bindings) -> Bindings {
    for action in Action::ALL {
        bindings
            .entry(action)
            .or_insert_with(|| action.default_binding().to_owned());
    }
    bindings
}

pub fn parse(binding: &str) -> Option<HotKey> {
    if binding.trim().is_empty() {
        return None;
    }
    binding.parse().ok()
}

/// Human-readable form, e.g. `"Ctrl+Alt+P"`. Empty bindings give `None`.
pub fn pretty(binding: &str) -> Option<String> {
    let hotkey = parse(binding)?;
    let mut parts = Vec::new();
    if hotkey.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_owned());
    }
    if hotkey.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_owned());
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_owned());
    }
    if hotkey.mods.contains(Modifiers::META) {
        parts.push("Win".to_owned());
    }
    parts.push(key_name(hotkey.key));
    Some(parts.join("+"))
}

fn key_name(code: Code) -> String {
    let name = code.to_string();
    if let Some(letter) = name.strip_prefix("Key") {
        return letter.to_owned();
    }
    if let Some(digit) = name.strip_prefix("Digit") {
        return digit.to_owned();
    }
    // Arrow keys in words: egui's bundled fonts have no ← → ↑ ↓ glyphs.
    match name.strip_prefix("Arrow") {
        Some(direction) => direction.to_owned(),
        None => name,
    }
}

/// Builds a binding from a key pressed in the app (used to change a hotkey).
/// Requires Ctrl or Alt, so plain typing keys can't become system-wide hotkeys.
pub fn binding_from_egui(modifiers: egui::Modifiers, key: egui::Key) -> Option<String> {
    if !(modifiers.ctrl || modifiers.alt) {
        return None;
    }
    let mut mods = Modifiers::empty();
    if modifiers.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if modifiers.alt {
        mods |= Modifiers::ALT;
    }
    if modifiers.shift {
        mods |= Modifiers::SHIFT;
    }
    Some(HotKey::new(Some(mods), code_for(key)?).into_string())
}

/// egui key → physical key code (US layout positions, as Windows hotkeys use).
fn code_for(key: egui::Key) -> Option<Code> {
    use egui::Key as K;
    Some(match key {
        K::A => Code::KeyA,
        K::B => Code::KeyB,
        K::C => Code::KeyC,
        K::D => Code::KeyD,
        K::E => Code::KeyE,
        K::F => Code::KeyF,
        K::G => Code::KeyG,
        K::H => Code::KeyH,
        K::I => Code::KeyI,
        K::J => Code::KeyJ,
        K::K => Code::KeyK,
        K::L => Code::KeyL,
        K::M => Code::KeyM,
        K::N => Code::KeyN,
        K::O => Code::KeyO,
        K::P => Code::KeyP,
        K::Q => Code::KeyQ,
        K::R => Code::KeyR,
        K::S => Code::KeyS,
        K::T => Code::KeyT,
        K::U => Code::KeyU,
        K::V => Code::KeyV,
        K::W => Code::KeyW,
        K::X => Code::KeyX,
        K::Y => Code::KeyY,
        K::Z => Code::KeyZ,
        K::Num0 => Code::Digit0,
        K::Num1 => Code::Digit1,
        K::Num2 => Code::Digit2,
        K::Num3 => Code::Digit3,
        K::Num4 => Code::Digit4,
        K::Num5 => Code::Digit5,
        K::Num6 => Code::Digit6,
        K::Num7 => Code::Digit7,
        K::Num8 => Code::Digit8,
        K::Num9 => Code::Digit9,
        K::F1 => Code::F1,
        K::F2 => Code::F2,
        K::F3 => Code::F3,
        K::F4 => Code::F4,
        K::F5 => Code::F5,
        K::F6 => Code::F6,
        K::F7 => Code::F7,
        K::F8 => Code::F8,
        K::F9 => Code::F9,
        K::F10 => Code::F10,
        K::F11 => Code::F11,
        K::F12 => Code::F12,
        K::ArrowUp => Code::ArrowUp,
        K::ArrowDown => Code::ArrowDown,
        K::ArrowLeft => Code::ArrowLeft,
        K::ArrowRight => Code::ArrowRight,
        K::PageUp => Code::PageUp,
        K::PageDown => Code::PageDown,
        K::Home => Code::Home,
        K::End => Code::End,
        K::Insert => Code::Insert,
        K::Delete => Code::Delete,
        K::Space => Code::Space,
        K::Enter => Code::Enter,
        K::Backspace => Code::Backspace,
        K::Minus => Code::Minus,
        K::Equals | K::Plus => Code::Equal,
        K::Comma => Code::Comma,
        K::Period => Code::Period,
        K::Slash => Code::Slash,
        K::Backslash => Code::Backslash,
        K::Semicolon => Code::Semicolon,
        K::Quote => Code::Quote,
        K::OpenBracket => Code::BracketLeft,
        K::CloseBracket => Code::BracketRight,
        K::Backtick => Code::Backquote,
        _ => return None,
    })
}

/// Why a binding is not active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    /// Another program already registered this combination.
    InUse,
    /// The same combination is assigned to another action.
    Duplicate,
    /// The stored text is not a valid combination.
    Invalid,
}

/// Actions whose (non-empty) binding is also used by another action.
pub fn duplicates(bindings: &Bindings) -> Vec<Action> {
    let mut by_hotkey: HashMap<u32, Vec<Action>> = HashMap::new();
    for (&action, binding) in bindings {
        if let Some(hotkey) = parse(binding) {
            by_hotkey.entry(hotkey.id()).or_default().push(action);
        }
    }
    let mut result: Vec<Action> = by_hotkey
        .into_values()
        .filter(|actions| actions.len() > 1)
        .flatten()
        .collect();
    result.sort();
    result
}

/// Registers the bindings with Windows and reports pressed hotkeys as actions.
/// Must be created on the main (event-loop) thread.
pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    registered: HashMap<u32, (Action, HotKey)>,
    errors: BTreeMap<Action, BindingError>,
    receiver: Receiver<u32>,
}

impl HotkeyService {
    /// `ctx` is woken on every hotkey press, so actions run even while the panel is
    /// minimized or hidden.
    pub fn new(ctx: &egui::Context) -> Self {
        let manager = GlobalHotKeyManager::new()
            .map_err(|err| log::error!("global hotkeys unavailable: {err}"))
            .ok();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = sender.send(event.id());
                ctx.request_repaint();
            }
        }));
        Self {
            manager,
            registered: HashMap::new(),
            errors: BTreeMap::new(),
            receiver,
        }
    }

    /// Replaces all registered hotkeys with `bindings`.
    pub fn apply(&mut self, bindings: &Bindings) {
        self.suspend();
        self.errors.clear();
        let Some(manager) = &self.manager else {
            return;
        };
        let duplicates = duplicates(bindings);
        for (&action, binding) in bindings {
            if binding.trim().is_empty() {
                continue;
            }
            if duplicates.contains(&action) {
                self.errors.insert(action, BindingError::Duplicate);
                continue;
            }
            let Some(hotkey) = parse(binding) else {
                self.errors.insert(action, BindingError::Invalid);
                continue;
            };
            match manager.register(hotkey) {
                Ok(()) => {
                    self.registered.insert(hotkey.id(), (action, hotkey));
                }
                Err(err) => {
                    log::warn!("hotkey {binding} for {action:?} not registered: {err}");
                    self.errors.insert(action, BindingError::InUse);
                }
            }
        }
    }

    /// Unregisters everything (e.g. while the user records a new combination, so the
    /// current hotkeys don't swallow the keys).
    pub fn suspend(&mut self) {
        if let Some(manager) = &self.manager {
            for (_, hotkey) in self.registered.values() {
                let _ = manager.unregister(*hotkey);
            }
        }
        self.registered.clear();
    }

    pub fn error(&self, action: Action) -> Option<BindingError> {
        self.errors.get(&action).copied()
    }

    /// Actions triggered since the last call.
    pub fn poll(&mut self) -> Vec<Action> {
        self.receiver
            .try_iter()
            .filter_map(|id| self.registered.get(&id).map(|(action, _)| *action))
            .collect()
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.suspend();
        GlobalHotKeyEvent::set_event_handler(None::<fn(GlobalHotKeyEvent)>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_every_action_and_parse() {
        let bindings = default_bindings();
        assert_eq!(bindings.len(), Action::ALL.len());
        for binding in bindings.values() {
            assert!(parse(binding).is_some(), "{binding} should parse");
        }
        assert!(duplicates(&bindings).is_empty());
    }

    #[test]
    fn pretty_names() {
        assert_eq!(pretty("control+alt+KeyP").as_deref(), Some("Ctrl+Alt+P"));
        assert_eq!(
            pretty("control+alt+PageUp").as_deref(),
            Some("Ctrl+Alt+PageUp")
        );
        assert_eq!(pretty("alt+shift+Digit5").as_deref(), Some("Alt+Shift+5"));
        assert_eq!(pretty("control+ArrowUp").as_deref(), Some("Ctrl+Up"));
        assert_eq!(pretty(""), None);
    }

    #[test]
    fn egui_keys_become_round_trippable_bindings() {
        let ctrl_alt = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        let binding = binding_from_egui(ctrl_alt, egui::Key::P).unwrap();
        assert_eq!(binding, "control+alt+KeyP");
        assert_eq!(parse(&binding).unwrap().into_string(), binding);

        let shift = egui::Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            binding_from_egui(shift, egui::Key::F5).as_deref(),
            Some("shift+control+F5")
        );
    }

    #[test]
    fn plain_keys_are_not_accepted_as_hotkeys() {
        let none = egui::Modifiers::default();
        let shift_only = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(binding_from_egui(none, egui::Key::P), None);
        assert_eq!(binding_from_egui(shift_only, egui::Key::P), None);
    }

    #[test]
    fn duplicate_bindings_are_reported() {
        let mut bindings = default_bindings();
        bindings.insert(Action::ShowPanel, "control+alt+KeyP".to_owned());
        assert_eq!(
            duplicates(&bindings),
            vec![Action::TogglePassthrough, Action::ShowPanel]
        );
    }

    #[test]
    fn missing_actions_get_defaults_and_empty_means_none() {
        let mut partial = Bindings::new();
        partial.insert(Action::ZoomIn, String::new());
        let filled = with_defaults(partial);
        assert_eq!(filled.len(), Action::ALL.len());
        assert_eq!(filled[&Action::ZoomIn], "", "cleared hotkey stays cleared");
        assert_eq!(parse(&filled[&Action::ZoomIn]), None);
        assert_eq!(filled[&Action::ToggleLupa], "control+alt+KeyL");
    }

    #[test]
    fn bindings_serialize_with_readable_keys() {
        let json = serde_json::to_string(&default_bindings()).unwrap();
        assert!(json.contains(r#""toggle_passthrough":"control+alt+KeyP""#));
    }

    /// Registers a real system-wide hotkey with Windows (tests run as a Windows process),
    /// using a combination nobody else is likely to use, then releases it.
    #[test]
    fn registers_a_real_hotkey_with_windows() {
        let ctx = egui::Context::default();
        let mut service = HotkeyService::new(&ctx);
        let mut bindings = Bindings::new();
        bindings.insert(Action::ZoomIn, "control+alt+shift+F12".to_owned());
        bindings.insert(Action::ZoomOut, "control+alt+shift+F12".to_owned());
        bindings.insert(Action::ShowPanel, "control+alt+shift+F11".to_owned());
        service.apply(&bindings);
        assert_eq!(
            service.error(Action::ShowPanel),
            None,
            "registration failed"
        );
        assert_eq!(service.error(Action::ZoomIn), Some(BindingError::Duplicate));
        assert!(service.poll().is_empty());

        // A combination already taken (here by `service`) is reported as in use.
        let mut other = HotkeyService::new(&ctx);
        let mut taken = Bindings::new();
        taken.insert(Action::ShowPanel, "control+alt+shift+F11".to_owned());
        other.apply(&taken);
        assert_eq!(other.error(Action::ShowPanel), Some(BindingError::InUse));
        drop(other);
        drop(service); // unregisters
    }
}
