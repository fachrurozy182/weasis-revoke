//! The main window and its parts: menus, docked panels, viewer tabs, indicators.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`.

mod dock;
mod dock_layout;
mod docking_keys;
mod main_window;
mod masking_indicator;
mod menu;
mod menus;
mod shortcut_page;
mod tab_stack;
mod viewer_menu;
mod viewers;
mod working_area;

pub use main_window::MainWindow;
pub use viewers::{Viewer, ViewerArea, Viewers};
