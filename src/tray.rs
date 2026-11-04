//! System tray icon with a menu, so the app keeps running when the panel is closed.
//!
//! The menu lists every magnifier (check mark = open) plus "Passthrough (all)", "Show
//! panel" and "Quit". It is rebuilt when anything shown in it changes. Menu clicks and
//! icon clicks become [`TrayCommand`]s, read with [`Tray::poll`] from `App::logic`
//! (which also runs while the panel is hidden).

use crate::icon;
use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, PoisonError};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowPanel,
    /// Open or close one magnifier (by id).
    ToggleMagnifier(u32),
    PassthroughAll,
    Quit,
}

/// Fixed menu texts (already translated, with hotkeys appended).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrayLabels {
    pub tooltip: String,
    pub show_panel: String,
    pub passthrough_all: String,
    pub quit: String,
}

/// One magnifier in the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMagnifier {
    pub id: u32,
    pub name: String,
    pub open: bool,
    /// Can be opened (has an area).
    pub has_area: bool,
}

/// App state mirrored in the menu.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayState {
    pub magnifiers: Vec<TrayMagnifier>,
    /// Every magnifier is in passthrough (check mark on "Passthrough (all)").
    pub all_passthrough: bool,
}

type CommandMap = Arc<Mutex<Vec<(MenuId, TrayCommand)>>>;

pub struct Tray {
    icon: TrayIcon,
    commands: CommandMap,
    receiver: Receiver<TrayCommand>,
    shown: Option<(TrayLabels, TrayState)>,
}

impl Tray {
    /// Creates the tray icon. Returns `None` (logged) if Windows refuses, in which case
    /// the app simply behaves as without a tray.
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let rgba = icon::app_icon_rgba(32)?;
        let tray_icon = tray_icon::Icon::from_rgba(rgba.rgba, rgba.width, rgba.height)
            .map_err(|err| log::error!("tray icon image: {err}"))
            .ok()?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(Menu::new()))
            .with_menu_on_left_click(false) // left click opens the panel
            .with_icon(tray_icon)
            .build()
            .map_err(|err| log::error!("tray icon: {err}"))
            .ok()?;

        let (sender, receiver) = mpsc::channel();
        let commands: CommandMap = Arc::default();
        let menu_commands = Arc::clone(&commands);
        let menu_sender = sender.clone();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let map = menu_commands.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, command)) = map.iter().find(|(id, _)| *id == event.id) {
                let _ = menu_sender.send(*command);
                menu_ctx.request_repaint();
            }
        }));
        let icon_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let open = matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            );
            if open {
                let _ = sender.send(TrayCommand::ShowPanel);
                icon_ctx.request_repaint();
            }
        }));

        Some(Self {
            icon,
            commands,
            receiver,
            shown: None,
        })
    }

    /// Rebuilds the menu when anything shown in it changed.
    pub fn update(&mut self, labels: &TrayLabels, state: &TrayState) {
        if self
            .shown
            .as_ref()
            .is_some_and(|(l, s)| l == labels && s == state)
        {
            return;
        }
        let menu = Menu::new();
        let mut map = Vec::new();

        let show_panel = MenuItem::new(&labels.show_panel, true, None);
        map.push((show_panel.id().clone(), TrayCommand::ShowPanel));
        let _ = menu.append(&show_panel);
        if !state.magnifiers.is_empty() {
            let _ = menu.append(&PredefinedMenuItem::separator());
        }
        for magnifier in &state.magnifiers {
            let item = CheckMenuItem::new(
                &magnifier.name,
                magnifier.has_area || magnifier.open,
                magnifier.open,
                None,
            );
            map.push((
                item.id().clone(),
                TrayCommand::ToggleMagnifier(magnifier.id),
            ));
            let _ = menu.append(&item);
        }
        if !state.magnifiers.is_empty() {
            let passthrough =
                CheckMenuItem::new(&labels.passthrough_all, true, state.all_passthrough, None);
            map.push((passthrough.id().clone(), TrayCommand::PassthroughAll));
            let _ = menu.append(&PredefinedMenuItem::separator());
            let _ = menu.append(&passthrough);
        }
        let _ = menu.append(&PredefinedMenuItem::separator());
        let quit = MenuItem::new(&labels.quit, true, None);
        map.push((quit.id().clone(), TrayCommand::Quit));
        let _ = menu.append(&quit);

        *self.commands.lock().unwrap_or_else(PoisonError::into_inner) = map;
        self.icon.set_menu(Some(Box::new(menu)));
        let _ = self.icon.set_tooltip(Some(&labels.tooltip));
        self.shown = Some((labels.clone(), state.clone()));
    }

    /// Commands since the last call.
    pub fn poll(&mut self) -> Vec<TrayCommand> {
        self.receiver.try_iter().collect()
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}
