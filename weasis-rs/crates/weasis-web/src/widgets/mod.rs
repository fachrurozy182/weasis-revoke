//! Base components of the interface, after the Swing components of Weasis with the FlatLaf look.
//!
//! They are styled by `style/widgets.css` from the theme variables (`crate::theme`).

mod button;
mod collapsible;
mod dialog;
mod dropdown;
mod slider;
mod toolbar;

pub use button::{Button, ButtonKind, ToggleButton};
pub use collapsible::CollapsiblePanel;
pub use dialog::Dialog;
pub use dropdown::{DropDownButton, MenuChoice};
pub use slider::Slider;
pub use toolbar::{Toolbar, ToolbarSeparator};
