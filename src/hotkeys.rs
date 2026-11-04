//! System-wide keyboard shortcuts (work while a game or another app has focus).
//!
//! Bindings are stored as strings like `"control+alt+KeyP"` (the `global-hotkey`
//! format). [`HotkeyService`] registers them with Windows and turns key presses into
//! [`Action`]s.
//!
//! Two layers: `RegisterHotKey` (through `global-hotkey`) reserves each combination and
//! reports conflicts, and a low-level keyboard hook in front of it does the matching, so
//! shortcuts can be chained while Ctrl+Alt stay held (Windows' own hotkeys fire only the
//! first one) and holding a zoom key keeps zooming. A key the hook matches is swallowed,
//! so it never reaches `RegisterHotKey` and nothing fires twice; if the hook is missing,
//! `RegisterHotKey` still works on its own.

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::PoisonError;
use std::time::{Duration, Instant};

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

/// Windows virtual-key code of a physical key (same table as `global-hotkey`, so the
/// hook and `RegisterHotKey` agree). Covers every key [`binding_from_egui`] produces.
pub fn vk_for(code: Code) -> Option<u32> {
    let letter = |c: u8| Some(u32::from(c));
    match code {
        Code::KeyA => letter(b'A'),
        Code::KeyB => letter(b'B'),
        Code::KeyC => letter(b'C'),
        Code::KeyD => letter(b'D'),
        Code::KeyE => letter(b'E'),
        Code::KeyF => letter(b'F'),
        Code::KeyG => letter(b'G'),
        Code::KeyH => letter(b'H'),
        Code::KeyI => letter(b'I'),
        Code::KeyJ => letter(b'J'),
        Code::KeyK => letter(b'K'),
        Code::KeyL => letter(b'L'),
        Code::KeyM => letter(b'M'),
        Code::KeyN => letter(b'N'),
        Code::KeyO => letter(b'O'),
        Code::KeyP => letter(b'P'),
        Code::KeyQ => letter(b'Q'),
        Code::KeyR => letter(b'R'),
        Code::KeyS => letter(b'S'),
        Code::KeyT => letter(b'T'),
        Code::KeyU => letter(b'U'),
        Code::KeyV => letter(b'V'),
        Code::KeyW => letter(b'W'),
        Code::KeyX => letter(b'X'),
        Code::KeyY => letter(b'Y'),
        Code::KeyZ => letter(b'Z'),
        Code::Digit0 => letter(b'0'),
        Code::Digit1 => letter(b'1'),
        Code::Digit2 => letter(b'2'),
        Code::Digit3 => letter(b'3'),
        Code::Digit4 => letter(b'4'),
        Code::Digit5 => letter(b'5'),
        Code::Digit6 => letter(b'6'),
        Code::Digit7 => letter(b'7'),
        Code::Digit8 => letter(b'8'),
        Code::Digit9 => letter(b'9'),
        Code::F1 => Some(0x70),
        Code::F2 => Some(0x71),
        Code::F3 => Some(0x72),
        Code::F4 => Some(0x73),
        Code::F5 => Some(0x74),
        Code::F6 => Some(0x75),
        Code::F7 => Some(0x76),
        Code::F8 => Some(0x77),
        Code::F9 => Some(0x78),
        Code::F10 => Some(0x79),
        Code::F11 => Some(0x7A),
        Code::F12 => Some(0x7B),
        Code::Backspace => Some(0x08),
        Code::Enter => Some(0x0D),
        Code::Space => Some(0x20),
        Code::PageUp => Some(0x21),
        Code::PageDown => Some(0x22),
        Code::End => Some(0x23),
        Code::Home => Some(0x24),
        Code::ArrowLeft => Some(0x25),
        Code::ArrowUp => Some(0x26),
        Code::ArrowRight => Some(0x27),
        Code::ArrowDown => Some(0x28),
        Code::Insert => Some(0x2D),
        Code::Delete => Some(0x2E),
        Code::Semicolon => Some(0xBA),
        Code::Equal => Some(0xBB),
        Code::Comma => Some(0xBC),
        Code::Minus => Some(0xBD),
        Code::Period => Some(0xBE),
        Code::Slash => Some(0xBF),
        Code::Backquote => Some(0xC0),
        Code::BracketLeft => Some(0xDB),
        Code::Backslash => Some(0xDC),
        Code::BracketRight => Some(0xDD),
        Code::Quote => Some(0xDE),
        _ => None,
    }
}

/// Modifier virtual keys (generic, left and right): never shortcuts on their own.
fn is_modifier_vk(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

/// Held zoom keys repeat at most this often (about 8 steps per second).
pub const ZOOM_REPEAT_INTERVAL: Duration = Duration::from_millis(100);

/// What the keyboard hook does with one key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Not ours: let it through to the focused app.
    Pass,
    /// Part of a shortcut (its key-up, or a repeat that doesn't fire): eat it.
    Swallow,
    /// Run the action and eat the key.
    Fire(Action),
}

/// Matches key events against the bindings. Each key press is matched on its own, so
/// several shortcuts can follow each other while the modifiers stay held. Pure logic
/// (no Windows calls), fed by the keyboard hook.
#[derive(Debug, Default)]
pub struct ChordMatcher {
    bindings: Vec<(Modifiers, u32, Action)>,
    /// Keys currently down (to tell OS auto-repeat from a new press).
    down: HashSet<u32>,
    /// Keys whose press was a shortcut: their repeats and key-up are eaten too.
    swallowed: HashSet<u32>,
    last_fire: HashMap<u32, Instant>,
}

impl ChordMatcher {
    /// Matches only these bindings from now on (invalid or empty ones are skipped).
    pub fn set_bindings<'a>(&mut self, bindings: impl IntoIterator<Item = (Action, &'a str)>) {
        self.bindings = bindings
            .into_iter()
            .filter_map(|(action, binding)| {
                let hotkey = parse(binding)?;
                Some((hotkey.mods, vk_for(hotkey.key)?, action))
            })
            .collect();
    }

    /// Matches nothing (while a new combination is being recorded).
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn key(&mut self, vk: u32, is_down: bool, mods: Modifiers, now: Instant) -> Decision {
        if is_modifier_vk(vk) {
            return Decision::Pass;
        }
        if !is_down {
            self.down.remove(&vk);
            self.last_fire.remove(&vk);
            return if self.swallowed.remove(&vk) {
                Decision::Swallow
            } else {
                Decision::Pass
            };
        }
        let repeat = !self.down.insert(vk);
        let action = self
            .bindings
            .iter()
            .find(|(m, k, _)| *k == vk && *m == mods)
            .map(|(_, _, action)| *action);
        match action {
            Some(action) if !repeat => {
                self.swallowed.insert(vk);
                self.last_fire.insert(vk, now);
                Decision::Fire(action)
            }
            Some(action @ (Action::ZoomIn | Action::ZoomOut)) => {
                self.swallowed.insert(vk);
                let due = self
                    .last_fire
                    .get(&vk)
                    .is_none_or(|last| now.duration_since(*last) >= ZOOM_REPEAT_INTERVAL);
                if due {
                    self.last_fire.insert(vk, now);
                    Decision::Fire(action)
                } else {
                    Decision::Swallow
                }
            }
            // Held key of another shortcut: fires once per press.
            Some(_) => Decision::Swallow,
            // A repeat of a key that started as a shortcut (modifiers changed since).
            None if self.swallowed.contains(&vk) => Decision::Swallow,
            None => Decision::Pass,
        }
    }
}

/// The low-level keyboard hook: one per process, on its own thread with a message loop
/// (so a busy UI frame can never make Windows drop the hook for being slow).
mod keyboard_hook {
    use super::{Action, ChordMatcher, Decision, Modifiers};
    use eframe::egui;
    use std::sync::mpsc::Sender;
    use std::sync::{Mutex, OnceLock, PoisonError};
    use std::time::Instant;
    use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_UP, MSG, WH_KEYBOARD_LL,
    };

    pub static MATCHER: Mutex<Option<ChordMatcher>> = Mutex::new(None);
    /// Where fired actions go (the app's [`super::HotkeyService`]).
    pub static SINK: Mutex<Option<(Sender<Action>, egui::Context)>> = Mutex::new(None);
    static STARTED: OnceLock<bool> = OnceLock::new();

    pub fn with_matcher(f: impl FnOnce(&mut ChordMatcher)) {
        let mut matcher = MATCHER.lock().unwrap_or_else(PoisonError::into_inner);
        f(matcher.get_or_insert_with(ChordMatcher::default));
    }

    /// Starts the hook thread once. Returns whether the hook is installed.
    pub fn ensure_started() -> bool {
        *STARTED.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new()
                .name("keyboard-hook".into())
                .spawn(move || unsafe {
                    let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
                    let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
                    let ok = hook.is_ok();
                    if let Err(err) = &hook {
                        log::error!("keyboard hook not installed: {err}");
                    }
                    let _ = tx.send(ok);
                    if !ok {
                        return;
                    }
                    // Lives as long as the process; the message loop runs the hook.
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                });
            spawned.is_ok() && rx.recv().unwrap_or(false)
        })
    }

    fn held(vk: i32) -> bool {
        unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
    }

    fn current_modifiers() -> Modifiers {
        let mut mods = Modifiers::empty();
        if held(0x11) {
            mods |= Modifiers::CONTROL;
        }
        if held(0x12) {
            mods |= Modifiers::ALT;
        }
        if held(0x10) {
            mods |= Modifiers::SHIFT;
        }
        if held(0x5B) || held(0x5C) {
            mods |= Modifiers::META;
        }
        mods
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let is_down = event.flags.0 & LLKHF_UP.0 == 0;
            let decision = {
                let mut matcher = MATCHER.lock().unwrap_or_else(PoisonError::into_inner);
                match matcher.as_mut() {
                    Some(m) => m.key(event.vkCode, is_down, current_modifiers(), Instant::now()),
                    None => Decision::Pass,
                }
            };
            match decision {
                Decision::Pass => {}
                Decision::Swallow => return LRESULT(1),
                Decision::Fire(action) => {
                    if let Some((sender, ctx)) =
                        SINK.lock().unwrap_or_else(PoisonError::into_inner).as_ref()
                    {
                        let _ = sender.send(action);
                        ctx.request_repaint();
                    }
                    return LRESULT(1);
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

/// Registers the bindings with Windows and reports pressed hotkeys as actions.
/// Must be created on the main (event-loop) thread. The app has one; the keyboard hook
/// reports to the most recently created service.
pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    registered: HashMap<u32, (Action, HotKey)>,
    errors: BTreeMap<Action, BindingError>,
    receiver: Receiver<u32>,
    /// Actions fired by the keyboard hook.
    hook_receiver: Receiver<Action>,
}

impl HotkeyService {
    /// `ctx` is woken on every hotkey press, so actions run even while the panel is
    /// minimized or hidden.
    pub fn new(ctx: &egui::Context) -> Self {
        let manager = GlobalHotKeyManager::new()
            .map_err(|err| log::error!("global hotkeys unavailable: {err}"))
            .ok();
        let (sender, receiver) = mpsc::channel();
        let ctx_for_hook = ctx.clone();
        let ctx = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = sender.send(event.id());
                ctx.request_repaint();
            }
        }));
        let (hook_sender, hook_receiver): (Sender<Action>, _) = mpsc::channel();
        if keyboard_hook::ensure_started() {
            *keyboard_hook::SINK
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some((hook_sender, ctx_for_hook));
        }
        Self {
            manager,
            registered: HashMap::new(),
            errors: BTreeMap::new(),
            receiver,
            hook_receiver,
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
        self.sync_matcher();
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
        keyboard_hook::with_matcher(ChordMatcher::clear);
    }

    /// Lets the keyboard hook match exactly what Windows accepted.
    fn sync_matcher(&self) {
        let bindings: Vec<(Action, String)> = self
            .registered
            .values()
            .map(|(action, hotkey)| (*action, hotkey.into_string()))
            .collect();
        keyboard_hook::with_matcher(|matcher| {
            matcher.set_bindings(bindings.iter().map(|(a, b)| (*a, b.as_str())));
        });
    }

    pub fn error(&self, action: Action) -> Option<BindingError> {
        self.errors.get(&action).copied()
    }

    /// Actions triggered since the last call.
    pub fn poll(&mut self) -> Vec<Action> {
        let mut actions: Vec<Action> = self
            .receiver
            .try_iter()
            .filter_map(|id| self.registered.get(&id).map(|(action, _)| *action))
            .collect();
        actions.extend(self.hook_receiver.try_iter());
        actions
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

    fn matcher(bindings: &[(Action, &str)]) -> ChordMatcher {
        let mut m = ChordMatcher::default();
        m.set_bindings(bindings.iter().copied());
        m
    }

    const CTRL_ALT: Modifiers = Modifiers::CONTROL.union(Modifiers::ALT);
    const VK_L: u32 = b'L' as u32;
    const VK_P: u32 = b'P' as u32;
    const VK_H: u32 = b'H' as u32;
    const VK_PAGE_UP: u32 = 0x21;
    const VK_CONTROL: u32 = 0xA2;

    fn defaults() -> ChordMatcher {
        let bindings = default_bindings();
        let mut m = ChordMatcher::default();
        m.set_bindings(bindings.iter().map(|(a, b)| (*a, b.as_str())));
        m
    }

    #[test]
    fn shortcuts_chain_while_modifiers_stay_held() {
        let mut m = defaults();
        let t = Instant::now();
        assert_eq!(
            m.key(VK_CONTROL, true, Modifiers::CONTROL, t),
            Decision::Pass
        );
        let mut fired = Vec::new();
        for vk in [VK_L, VK_P, VK_H] {
            if let Decision::Fire(action) = m.key(vk, true, CTRL_ALT, t) {
                fired.push(action);
            }
            assert_eq!(
                m.key(vk, false, CTRL_ALT, t),
                Decision::Swallow,
                "key-up is eaten"
            );
        }
        assert_eq!(
            fired,
            [
                Action::ToggleLupa,
                Action::TogglePassthrough,
                Action::ShowPanel
            ]
        );
    }

    #[test]
    fn same_shortcut_fires_on_every_press() {
        let mut m = matcher(&[(Action::TogglePassthrough, "control+alt+KeyP")]);
        let t = Instant::now();
        for _ in 0..3 {
            assert_eq!(
                m.key(VK_P, true, CTRL_ALT, t),
                Decision::Fire(Action::TogglePassthrough)
            );
            m.key(VK_P, false, CTRL_ALT, t);
        }
    }

    #[test]
    fn held_zoom_repeats_and_other_shortcuts_fire_once() {
        let mut m = defaults();
        let t = Instant::now();
        let ms = |n| t + Duration::from_millis(n);
        assert_eq!(
            m.key(VK_PAGE_UP, true, CTRL_ALT, t),
            Decision::Fire(Action::ZoomIn)
        );
        // OS auto-repeat every ~33 ms: fires every 100 ms.
        let fired = (1..=9)
            .filter(|i| {
                m.key(VK_PAGE_UP, true, CTRL_ALT, ms(i * 34)) == Decision::Fire(Action::ZoomIn)
            })
            .count();
        assert_eq!(fired, 3);
        assert_eq!(
            m.key(VK_PAGE_UP, false, CTRL_ALT, ms(400)),
            Decision::Swallow
        );

        assert_eq!(
            m.key(VK_P, true, CTRL_ALT, t),
            Decision::Fire(Action::TogglePassthrough)
        );
        for i in 1..5 {
            assert_eq!(m.key(VK_P, true, CTRL_ALT, ms(i * 200)), Decision::Swallow);
        }
    }

    #[test]
    fn other_keys_and_modifiers_pass_through() {
        let mut m = defaults();
        let t = Instant::now();
        assert_eq!(m.key(VK_P, true, Modifiers::CONTROL, t), Decision::Pass);
        assert_eq!(m.key(VK_P, false, Modifiers::CONTROL, t), Decision::Pass);
        let shift = CTRL_ALT | Modifiers::SHIFT;
        assert_eq!(m.key(VK_P, true, shift, t), Decision::Pass);
        assert_eq!(m.key(VK_P, false, shift, t), Decision::Pass);
        assert_eq!(m.key(b'Q' as u32, true, CTRL_ALT, t), Decision::Pass);
        m.clear();
        assert_eq!(m.key(VK_P, true, CTRL_ALT, t), Decision::Pass);
    }

    /// Every key a hotkey can be recorded with has a virtual-key code.
    #[test]
    fn every_recordable_key_has_a_virtual_key() {
        for key in egui::Key::ALL {
            if let Some(binding) = binding_from_egui(egui::Modifiers::CTRL, *key) {
                let hotkey = parse(&binding).unwrap();
                assert!(vk_for(hotkey.key).is_some(), "{binding} has no virtual key");
            }
        }
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
        assert!(keyboard_hook::ensure_started(), "keyboard hook installs");
        drop(service); // unregisters
    }
}
