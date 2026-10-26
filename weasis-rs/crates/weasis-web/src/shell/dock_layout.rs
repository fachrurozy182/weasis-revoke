//! Where the dockable panels are: their side, their state and the size of each side.
//!
//! A dockable is docked (shown on its side, one tab among the panels of that side), minimized
//! (a button on the side strip that shows it on demand) or closed (only reopened from the
//! View menu). The layout is saved in the browser (decision D8).
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/docking/PluginTool.java` and
//! `DockableTool.java` (DockingFrames `DefaultSingleCDockable` with its extended modes).

use serde::{Deserialize, Serialize};

/// Smallest and largest width (or height for the bottom side) of a side, in pixels.
const SIZE_RANGE: (f64, f64) = (160.0, 900.0);
/// Size of a side before the user resizes it.
const DEFAULT_SIZE: f64 = 300.0;

/// A side of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockSide {
    /// West: the explorers.
    Left,
    /// East: the tools of the viewer.
    Right,
    /// South.
    Bottom,
}

impl DockSide {
    /// The three sides.
    pub const ALL: [Self; 3] = [Self::Left, Self::Right, Self::Bottom];

    fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Bottom => 2,
        }
    }
}

/// State of a dockable (DockingFrames extended modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockMode {
    /// Shown on its side.
    Docked,
    /// A button on the side strip.
    Minimized,
    /// Hidden.
    Closed,
}

/// Placement of one dockable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockPlacement {
    /// Stable identifier.
    pub id: String,
    /// Side it is on.
    pub side: DockSide,
    /// State.
    pub mode: DockMode,
}

/// The layout of all dockables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    placements: Vec<DockPlacement>,
    sizes: [f64; 3],
    active: [Option<String>; 3],
}

impl Default for DockLayout {
    fn default() -> Self {
        Self {
            placements: Vec::new(),
            sizes: [DEFAULT_SIZE; 3],
            active: [None, None, None],
        }
    }
}

impl DockLayout {
    /// Adds a dockable with its default place; a dockable already known keeps its place.
    pub fn register(&mut self, id: &str, side: DockSide, mode: DockMode) {
        if self.placement(id).is_none() {
            self.placements.push(DockPlacement {
                id: id.to_owned(),
                side,
                mode,
            });
            if mode == DockMode::Docked && self.active[side.index()].is_none() {
                self.active[side.index()] = Some(id.to_owned());
            }
        }
    }

    /// Placement of a dockable.
    pub fn placement(&self, id: &str) -> Option<&DockPlacement> {
        self.placements.iter().find(|p| p.id == id)
    }

    /// Ids of the dockables of a side in a mode, in registration order.
    pub fn on_side(&self, side: DockSide, mode: DockMode) -> Vec<&str> {
        self.placements
            .iter()
            .filter(|p| p.side == side && p.mode == mode)
            .map(|p| p.id.as_str())
            .collect()
    }

    /// The tab shown on a side.
    pub fn active(&self, side: DockSide) -> Option<&str> {
        self.active[side.index()].as_deref()
    }

    /// Width (or height) of a side.
    pub fn size(&self, side: DockSide) -> f64 {
        self.sizes[side.index()]
    }

    /// Docks a dockable on its side and shows its tab.
    pub fn show(&mut self, id: &str) {
        if let Some(side) = self.set_mode(id, DockMode::Docked) {
            self.active[side.index()] = Some(id.to_owned());
        }
    }

    /// Puts a dockable on the side strip.
    pub fn minimize(&mut self, id: &str) {
        self.set_mode(id, DockMode::Minimized);
    }

    /// Hides a dockable.
    pub fn close(&mut self, id: &str) {
        self.set_mode(id, DockMode::Closed);
    }

    /// Moves a dockable to another side, docked and shown there.
    pub fn move_to(&mut self, id: &str, side: DockSide) {
        if let Some(placement) = self.placements.iter_mut().find(|p| p.id == id) {
            placement.side = side;
        }
        self.show(id);
        self.fix_active();
    }

    /// Resizes a side, within the allowed range.
    pub fn resize(&mut self, side: DockSide, size: f64) {
        self.sizes[side.index()] = size.clamp(SIZE_RANGE.0, SIZE_RANGE.1);
    }

    /// Changes the mode; returns the side of the dockable.
    fn set_mode(&mut self, id: &str, mode: DockMode) -> Option<DockSide> {
        let placement = self.placements.iter_mut().find(|p| p.id == id)?;
        placement.mode = mode;
        let side = placement.side;
        self.fix_active();
        Some(side)
    }

    /// Each side shows one of its docked dockables, if it has any.
    fn fix_active(&mut self) {
        for side in DockSide::ALL {
            let docked = self.on_side(side, DockMode::Docked);
            let current = self.active[side.index()].as_deref();
            if !current.is_some_and(|id| docked.contains(&id)) {
                let first = docked.first().map(|id| (*id).to_owned());
                self.active[side.index()] = first;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> DockLayout {
        let mut layout = DockLayout::default();
        layout.register("explorer", DockSide::Left, DockMode::Docked);
        layout.register("display", DockSide::Right, DockMode::Docked);
        layout.register("measure", DockSide::Right, DockMode::Docked);
        layout
    }

    #[test]
    fn the_first_docked_panel_of_a_side_is_shown() {
        let layout = layout();
        assert_eq!(layout.active(DockSide::Left), Some("explorer"));
        assert_eq!(layout.active(DockSide::Right), Some("display"));
        assert_eq!(
            layout.on_side(DockSide::Right, DockMode::Docked),
            ["display", "measure"]
        );
        assert_eq!(layout.active(DockSide::Bottom), None);
    }

    #[test]
    fn minimizing_closing_and_showing_update_the_tabs() {
        let mut layout = layout();
        layout.minimize("display");
        assert_eq!(layout.active(DockSide::Right), Some("measure"));
        assert_eq!(
            layout.on_side(DockSide::Right, DockMode::Minimized),
            ["display"]
        );
        layout.close("measure");
        assert_eq!(layout.active(DockSide::Right), None);
        layout.show("display");
        assert_eq!(layout.active(DockSide::Right), Some("display"));
    }

    #[test]
    fn a_panel_moves_to_another_side() {
        let mut layout = layout();
        layout.move_to("measure", DockSide::Left);
        assert_eq!(
            layout.on_side(DockSide::Left, DockMode::Docked),
            ["explorer", "measure"]
        );
        assert_eq!(layout.active(DockSide::Left), Some("measure"));
        assert_eq!(layout.active(DockSide::Right), Some("display"));
    }

    #[test]
    fn sizes_stay_in_range_and_the_layout_round_trips() {
        let mut layout = layout();
        layout.resize(DockSide::Left, 20.0);
        assert_eq!(layout.size(DockSide::Left), 160.0);
        layout.register("explorer", DockSide::Right, DockMode::Closed);
        assert_eq!(
            layout.placement("explorer").map(|p| p.side),
            Some(DockSide::Left),
            "kept"
        );
        let json = serde_json::to_string(&layout).expect("serializable");
        assert_eq!(serde_json::from_str::<DockLayout>(&json).ok(), Some(layout));
    }
}
