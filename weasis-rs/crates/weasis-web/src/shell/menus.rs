//! The menus of the main window.
//!
//! The Open, Import, Export, Print and Toolbars submenus are filled by the explorers and viewers
//! of later steps (2.4, 2.5, 3.x, 7.x); the menu of the selected viewer comes with the viewers.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`
//! (`createMenuBar`, `buildMenuFile`, `buildMenuView`, `buildMenuTools`).

use leptos::prelude::*;

use super::menu::{Menu, MenuItem};
use crate::i18n::tr;

/// Ids of the actions of the main menus.
pub mod action {
    /// Opens the preferences.
    pub const PREFERENCES: &str = "preferences";
    /// Session identity masking switched.
    pub const ANONYMIZE: &str = "anonymize";
    /// Shows the keyboard shortcuts.
    pub const SHORTCUTS: &str = "shortcuts";
    /// Opens the online help.
    pub const ONLINE_HELP: &str = "online-help";
    /// Opens the issue tracker.
    pub const BUG_REPORT: &str = "bug-report";
    /// Shows the system resources.
    pub const RESOURCES: &str = "resources";
    /// Shows the about box.
    pub const ABOUT: &str = "about";
}

/// The menus File, View, Tools and Help.
///
/// NOTE: Exit, Check for Updates and Open the logging folder of the desktop application have no
/// meaning in a browser tab and are left out.
pub fn main_menus(anonymize: RwSignal<bool>) -> Vec<Menu> {
    let action = |id, key: &str, accelerator| MenuItem::Action {
        id,
        label: tr(key).to_owned(),
        accelerator,
    };
    let submenu = |key: &str| MenuItem::Submenu {
        label: tr(key).to_owned(),
        items: Vec::new(),
    };
    vec![
        Menu {
            label: tr("WeasisWin.file").to_owned(),
            items: vec![
                submenu("WeasisWin.open"),
                submenu("WeasisWin.import"),
                submenu("WeasisWin.export"),
                submenu("launcher"),
                MenuItem::Separator,
                submenu("WeasisWin.print"),
                MenuItem::Separator,
                action(
                    action::PREFERENCES,
                    "OpenPreferencesAction.title",
                    Some("Alt+P"),
                ),
            ],
        },
        Menu {
            label: tr("WeasisWin.display").to_owned(),
            items: vec![
                submenu("WeasisWin.toolbar"),
                submenu("WeasisWin.tools"),
                MenuItem::Submenu {
                    label: "Explorer".to_owned(),
                    items: Vec::new(),
                },
            ],
        },
        Menu {
            label: tr("WeasisWin.tools").to_owned(),
            items: vec![MenuItem::Check {
                id: action::ANONYMIZE,
                label: tr("WeasisWin.anonymize").to_owned(),
                title: Some(tr("WeasisWin.anonymize.tip").to_owned()),
                checked: anonymize,
            }],
        },
        Menu {
            label: tr("WeasisWin.help").to_owned(),
            items: vec![
                action(action::SHORTCUTS, "WeasisWin.shortcuts", None),
                action(action::ONLINE_HELP, "WeasisWin.online", None),
                MenuItem::Separator,
                action(action::BUG_REPORT, "submit.bug.report", None),
                MenuItem::Separator,
                action(action::RESOURCES, "ResourceMonitor.title", None),
                MenuItem::Action {
                    id: action::ABOUT,
                    label: tr("WeasisAboutBox.about").replace("%s", "Weasis"),
                    accelerator: None,
                },
            ],
        },
    ]
}
