//! System tray icon with a menu, so the app keeps running when the panel is closed.
//!
//! Menu clicks and icon clicks become [`TrayCommand`]s, read with [`Tray::poll`] from
//! `App::logic` (which also runs while the panel is hidden).

use crate::icon;
use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowPanel,
    ToggleLupa,
    TogglePassthrough,
    SelectArea,
    Quit,
}

/// Menu texts (already translated, with hotkeys appended).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrayLabels {
    pub tooltip: String,
    pub show_panel: String,
    pub open_lupa: String,
    pub close_lupa: String,
    pub passthrough: String,
    pub select_area: String,
    pub quit: String,
}

/// App state mirrored in the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrayState {
    pub has_area: bool,
    pub lupa_open: bool,
    pub passthrough: bool,
}

pub struct Tray {
    icon: TrayIcon,
    show_panel: MenuItem,
    lupa: MenuItem,
    passthrough: CheckMenuItem,
    select_area: MenuItem,
    quit: MenuItem,
    receiver: Receiver<TrayCommand>,
    shown: Option<(TrayLabels, TrayState)>,
}

impl Tray {
    /// Creates the tray icon. Returns `None` (logged) if Windows refuses, in which case
    /// the app simply behaves as without a tray.
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let show_panel = MenuItem::new("", true, None);
        let lupa = MenuItem::new("", true, None);
        let passthrough = CheckMenuItem::new("", true, false, None);
        let select_area = MenuItem::new("", true, None);
        let quit = MenuItem::new("", true, None);
        let menu = Menu::with_items(&[
            &show_panel,
            &PredefinedMenuItem::separator(),
            &lupa,
            &passthrough,
            &select_area,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|err| log::error!("tray menu: {err}"))
        .ok()?;

        let rgba = icon::app_icon_rgba(32)?;
        let tray_icon = tray_icon::Icon::from_rgba(rgba.rgba, rgba.width, rgba.height)
            .map_err(|err| log::error!("tray icon image: {err}"))
            .ok()?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false) // left click opens the panel
            .with_icon(tray_icon)
            .build()
            .map_err(|err| log::error!("tray icon: {err}"))
            .ok()?;

        let (sender, receiver) = mpsc::channel();
        let ids: Vec<(MenuId, TrayCommand)> = vec![
            (show_panel.id().clone(), TrayCommand::ShowPanel),
            (lupa.id().clone(), TrayCommand::ToggleLupa),
            (passthrough.id().clone(), TrayCommand::TogglePassthrough),
            (select_area.id().clone(), TrayCommand::SelectArea),
            (quit.id().clone(), TrayCommand::Quit),
        ];
        let menu_sender = sender.clone();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, command)) = ids.iter().find(|(id, _)| *id == event.id) {
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
            show_panel,
            lupa,
            passthrough,
            select_area,
            quit,
            receiver,
            shown: None,
        })
    }

    /// Updates texts, check mark and enabled states (only when something changed).
    pub fn update(&mut self, labels: &TrayLabels, state: TrayState) {
        if self
            .shown
            .as_ref()
            .is_some_and(|(l, s)| l == labels && *s == state)
        {
            return;
        }
        let _ = self.icon.set_tooltip(Some(&labels.tooltip));
        self.show_panel.set_text(&labels.show_panel);
        self.lupa.set_text(if state.lupa_open {
            &labels.close_lupa
        } else {
            &labels.open_lupa
        });
        self.lupa.set_enabled(state.has_area || state.lupa_open);
        self.passthrough.set_text(&labels.passthrough);
        self.passthrough.set_checked(state.passthrough);
        self.select_area.set_text(&labels.select_area);
        self.quit.set_text(&labels.quit);
        self.shown = Some((labels.clone(), state));
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
