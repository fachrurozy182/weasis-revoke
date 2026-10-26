//! The shortcuts of the application with their default keys.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/ShortcutManager.java`
//! (`ID_*` constants, `registerViewerShortcuts`).

use super::key_binding::{KeyBinding, key, modifier};
use super::shortcut_manager::{ShortcutContext, ShortcutManager};

/// Stable shortcut ids, used by the code, the preferences and the documentation.
pub mod id {
    #![allow(missing_docs)] // Named after the ID_* constants of ShortcutManager.java.
    pub const CINE_START: &str = "cinestart";
    pub const VIEWER_PRINT: &str = "viewer.print";
    pub const VIEWER_SCREENSHOT: &str = "viewer.screenshot";
    pub const VIEWER_EXPORT_ANIMATION: &str = "viewer.exportAnimation";
    pub const VIEWER_RECORD: &str = "viewer.record";
    pub const VIEWER_ESCAPE: &str = "viewer.escape";
    pub const VIEWER_ZOOM_OUT: &str = "viewer.zoomOut";
    pub const VIEWER_ZOOM_IN: &str = "viewer.zoomIn";
    pub const VIEWER_BEST_FIT: &str = "viewer.bestFit";
    pub const VIEWER_ROTATE_LEFT: &str = "viewer.rotateLeft";
    pub const VIEWER_ROTATE_RIGHT: &str = "viewer.rotateRight";
    pub const VIEWER_FLIP_HORIZONTAL: &str = "viewer.flipHorizontal";
    pub const VIEWER_SCROLL_UP: &str = "viewer.scrollUp";
    pub const VIEWER_SCROLL_DOWN: &str = "viewer.scrollDown";
    pub const VIEWER_SCROLL_UP_FAST: &str = "viewer.scrollUpFast";
    pub const VIEWER_SCROLL_DOWN_FAST: &str = "viewer.scrollDownFast";
    pub const VIEWER_SCROLL_FIRST: &str = "viewer.scrollFirst";
    pub const VIEWER_SCROLL_LAST: &str = "viewer.scrollLast";
    pub const VIEWER_NEXT_MOUSE_ACTION: &str = "viewer.nextMouseAction";
    pub const VIEWER_NEXT_VIEW: &str = "viewer.nextView";
    pub const VIEWER_PREV_VIEW: &str = "viewer.prevView";
    pub const VIEWER_TOGGLE_INFO: &str = "viewer.toggleInfo";
    pub const VIEWER_TOGGLE_INFO_ALT: &str = "viewer.toggleInfoAlt";
    pub const VIEWER_FULLSCREEN: &str = "viewer.fullscreen";
    pub const GRAPHIC_LINE: &str = "graphic.line";
    pub const GRAPHIC_ANGLE: &str = "graphic.angle";
    pub const GRAPHIC_POLYGON: &str = "graphic.polygon";
    pub const GRAPHIC_ANNOTATION: &str = "graphic.annotation";
    pub const DRAW_DELETE: &str = "draw.delete";
    pub const DRAW_DESELECT_ALL: &str = "draw.deselectAll";
    pub const DRAW_SELECT_ALL: &str = "draw.selectAll";
    pub const DRAW_CANCEL_DRAWING: &str = "draw.cancelDrawing";
    pub const DRAW_REMOVE_LAST_VERTEX: &str = "draw.removeLastVertex";
    pub const DRAW_COPY: &str = "draw.copy";
    pub const DRAW_CUT: &str = "draw.cut";
    pub const DRAW_PASTE: &str = "draw.paste";
    pub const DRAW_PASTE_IN_PLACE: &str = "draw.pasteInPlace";
    pub const DRAW_DUPLICATE: &str = "draw.duplicate";
    pub const PAN_LEFT: &str = "pan.left";
    pub const PAN_RIGHT: &str = "pan.right";
    pub const PAN_UP: &str = "pan.up";
    pub const PAN_DOWN: &str = "pan.down";
    pub const PAN_LEFT_FAST: &str = "pan.leftFast";
    pub const PAN_RIGHT_FAST: &str = "pan.rightFast";
    pub const PAN_UP_FAST: &str = "pan.upFast";
    pub const PAN_DOWN_FAST: &str = "pan.downFast";
    pub const DICOM_PREV_SERIES: &str = "dicom.prevSeries";
    pub const DICOM_NEXT_SERIES: &str = "dicom.nextSeries";
    pub const DICOM_PREV_STUDY: &str = "dicom.prevStudy";
    pub const DICOM_NEXT_STUDY: &str = "dicom.nextStudy";
    pub const DICOM_PREV_PATIENT: &str = "dicom.prevPatient";
    pub const DICOM_NEXT_PATIENT: &str = "dicom.nextPatient";
    pub const DICOM_FIRST_SERIES: &str = "dicom.firstSeries";
    pub const DICOM_LAST_SERIES: &str = "dicom.lastSeries";
    pub const DICOM_FIRST_STUDY: &str = "dicom.firstStudy";
    pub const DICOM_LAST_STUDY: &str = "dicom.lastStudy";
    pub const DICOM_FIRST_PATIENT: &str = "dicom.firstPatient";
    pub const DICOM_LAST_PATIENT: &str = "dicom.lastPatient";
    pub const DICOM_TOGGLE_SEG: &str = "dicom.toggleSegmentation";
    pub const DOCKING_MAXIMIZE: &str = "docking.maximize";
    pub const DOCKING_EXTERNALIZE: &str = "docking.externalize";
    pub const DOCKING_NORMALIZE: &str = "docking.normalize";
    pub const DOCKING_CLOSE: &str = "docking.close";
    pub const DOCKING_PANEL_LIST: &str = "docking.panelList";
    pub const DOCKING_NEXT_TAB: &str = "docking.nextTab";
    pub const DOCKING_PREV_TAB: &str = "docking.prevTab";
    pub const MPR_RECENTER: &str = "mpr.recenter";
    pub const MPR_RECENTER_ALL: &str = "mpr.recenterAll";
    pub const MPR_TOGGLE_CENTER: &str = "mpr.toggleCenter";
    pub const MPR_TOGGLE_CENTER_ALL: &str = "mpr.toggleCenterAll";
    pub const MPR_TOGGLE_CROSS_LINES: &str = "mpr.toggleCrossLines";
    pub const MPR_TOGGLE_CROSS_LINES_ALL: &str = "mpr.toggleCrossLinesAll";
    pub const MPR_CYCLE_MIP: &str = "mpr.cycleMip";
    pub const EXPLORER_OPEN: &str = "explorer.open";
    pub const EXPLORER_SELECT_NEXT: &str = "explorer.selectNext";
    pub const EXPLORER_SELECT_PREVIOUS: &str = "explorer.selectPrevious";
    pub const EXPLORER_SELECT_FIRST: &str = "explorer.selectFirst";
    pub const EXPLORER_SELECT_FIRST_ALT: &str = "explorer.selectFirstAlt";
    pub const EXPLORER_SELECT_LAST: &str = "explorer.selectLast";
    pub const EXPLORER_SELECT_LAST_ALT: &str = "explorer.selectLastAlt";
    pub const EXPLORER_SELECT_ALL: &str = "explorer.selectAll";
}

use ShortcutContext::{DicomExplorer, DicomViewer, Mpr, ViewCanvas};
use id::*;
use modifier::{ALT, CTRL, SHIFT};

/// Stands for the platform menu modifier ([`modifier::menu_shortcut`]) in the table below; it is
/// outside the keyboard modifier bits.
const MENU: u32 = 1 << 31;

/// Id, description (`ShortcutManager.*` message key), category (`ShortcutManager.cat.*`),
/// context, key code, modifiers.
type DefaultShortcut = (
    &'static str,
    &'static str,
    &'static str,
    ShortcutContext,
    u32,
    u32,
);

#[rustfmt::skip]
const DEFAULTS: [DefaultShortcut; 77] = [
    (CINE_START, "cine_start_stop", "mouse_actions", ViewCanvas, 'C' as u32, 0),
    (VIEWER_PRINT, "print_views", "viewer", ViewCanvas, 'P' as u32, 0),
    (VIEWER_SCREENSHOT, "screenshot", "viewer", ViewCanvas, 'S' as u32, MENU | SHIFT),
    (VIEWER_EXPORT_ANIMATION, "export_animation", "viewer", ViewCanvas, 'A' as u32, MENU | SHIFT),
    (VIEWER_RECORD, "record", "viewer", ViewCanvas, 'R' as u32, MENU | SHIFT),
    (VIEWER_ESCAPE, "reset_view", "viewer", ViewCanvas, key::ESCAPE, 0),
    (VIEWER_ZOOM_OUT, "zoom_out", "viewer", ViewCanvas, key::SUBTRACT, CTRL),
    (VIEWER_ZOOM_IN, "zoom_in", "viewer", ViewCanvas, key::ADD, CTRL),
    (VIEWER_BEST_FIT, "best_fit", "viewer", ViewCanvas, key::ENTER, CTRL),
    (VIEWER_SCROLL_UP, "prev_image", "navigation", ViewCanvas, key::UP, 0),
    (VIEWER_SCROLL_DOWN, "next_image", "navigation", ViewCanvas, key::DOWN, 0),
    (VIEWER_SCROLL_UP_FAST, "back_10", "navigation", ViewCanvas, key::UP, SHIFT),
    (VIEWER_SCROLL_DOWN_FAST, "fwd_10", "navigation", ViewCanvas, key::DOWN, SHIFT),
    (VIEWER_SCROLL_FIRST, "first_image", "navigation", ViewCanvas, key::HOME, 0),
    (VIEWER_SCROLL_LAST, "last_image", "navigation", ViewCanvas, key::END, 0),
    (VIEWER_NEXT_VIEW, "next_view", "navigation", ViewCanvas, key::TAB, 0),
    (VIEWER_PREV_VIEW, "prev_view", "navigation", ViewCanvas, key::TAB, SHIFT),
    (VIEWER_NEXT_MOUSE_ACTION, "next_mouse_action", "mouse_actions", ViewCanvas, key::SPACE, CTRL),
    (VIEWER_TOGGLE_INFO, "toggle_info", "display", ViewCanvas, key::SPACE, 0),
    (VIEWER_TOGGLE_INFO_ALT, "toggle_info", "display", ViewCanvas, 'I' as u32, 0),
    (VIEWER_FULLSCREEN, "fullscreen", "display", ViewCanvas, key::F1 + 10, 0),
    (VIEWER_ROTATE_LEFT, "rotate_left", "viewer", ViewCanvas, 'L' as u32, ALT),
    (VIEWER_ROTATE_RIGHT, "rotate_right", "viewer", ViewCanvas, 'R' as u32, ALT),
    (VIEWER_FLIP_HORIZONTAL, "flip_horiz", "viewer", ViewCanvas, 'F' as u32, ALT),
    (DRAW_DELETE, "delete_graphics", "drawings", ViewCanvas, key::DELETE, 0),
    (DRAW_DESELECT_ALL, "deselect_all", "drawings", ViewCanvas, 'D' as u32, CTRL),
    (DRAW_SELECT_ALL, "select_all_graphics", "drawings", ViewCanvas, 'A' as u32, CTRL),
    (DRAW_CANCEL_DRAWING, "cancel_drawing", "drawings", ViewCanvas, key::ESCAPE, 0),
    (DRAW_REMOVE_LAST_VERTEX, "remove_last_vertex", "drawings", ViewCanvas, key::BACK_SPACE, 0),
    (DRAW_COPY, "copy_graphics", "drawings", ViewCanvas, 'C' as u32, MENU),
    (DRAW_CUT, "cut_graphics", "drawings", ViewCanvas, 'X' as u32, MENU),
    (DRAW_PASTE, "paste_graphics", "drawings", ViewCanvas, 'V' as u32, MENU),
    (DRAW_PASTE_IN_PLACE, "paste_graphics_in_place", "drawings", ViewCanvas, 'V' as u32,
        MENU | SHIFT),
    (DRAW_DUPLICATE, "duplicate_graphics", "drawings", ViewCanvas, 'D' as u32, MENU | SHIFT),
    (PAN_LEFT, "pan_left_5", "pan", ViewCanvas, key::LEFT, ALT),
    (PAN_RIGHT, "pan_right_5", "pan", ViewCanvas, key::RIGHT, ALT),
    (PAN_UP, "pan_up_5", "pan", ViewCanvas, key::UP, ALT),
    (PAN_DOWN, "pan_down_5", "pan", ViewCanvas, key::DOWN, ALT),
    (PAN_LEFT_FAST, "pan_left_10", "pan", ViewCanvas, key::LEFT, ALT | SHIFT),
    (PAN_RIGHT_FAST, "pan_right_10", "pan", ViewCanvas, key::RIGHT, ALT | SHIFT),
    (PAN_UP_FAST, "pan_up_10", "pan", ViewCanvas, key::UP, ALT | SHIFT),
    (PAN_DOWN_FAST, "pan_down_10", "pan", ViewCanvas, key::DOWN, ALT | SHIFT),
    (DICOM_PREV_SERIES, "prev_series", "dicom_nav", DicomViewer, key::LEFT, 0),
    (DICOM_NEXT_SERIES, "next_series", "dicom_nav", DicomViewer, key::RIGHT, 0),
    (DICOM_PREV_STUDY, "prev_study", "dicom_nav", DicomViewer, key::LEFT, CTRL),
    (DICOM_NEXT_STUDY, "next_study", "dicom_nav", DicomViewer, key::RIGHT, CTRL),
    (DICOM_PREV_PATIENT, "prev_patient", "dicom_nav", DicomViewer, key::UP, CTRL),
    (DICOM_NEXT_PATIENT, "next_patient", "dicom_nav", DicomViewer, key::DOWN, CTRL),
    (DICOM_FIRST_SERIES, "first_series", "dicom_nav", DicomViewer, key::PAGE_UP, 0),
    (DICOM_LAST_SERIES, "last_series", "dicom_nav", DicomViewer, key::PAGE_DOWN, 0),
    (DICOM_FIRST_STUDY, "first_study", "dicom_nav", DicomViewer, key::PAGE_UP, CTRL),
    (DICOM_LAST_STUDY, "last_study", "dicom_nav", DicomViewer, key::PAGE_DOWN, CTRL),
    (DICOM_FIRST_PATIENT, "first_patient", "dicom_nav", DicomViewer, key::HOME, CTRL),
    (DICOM_LAST_PATIENT, "last_patient", "dicom_nav", DicomViewer, key::END, CTRL),
    (DICOM_TOGGLE_SEG, "toggle_seg", "display", DicomViewer, 'S' as u32, ALT),
    (MPR_RECENTER, "mpr_recenter", "mpr", Mpr, 'X' as u32, ALT),
    (MPR_RECENTER_ALL, "mpr_recenter_all", "mpr", Mpr, 'X' as u32, CTRL | ALT),
    (MPR_TOGGLE_CENTER, "mpr_toggle_center", "mpr", Mpr, 'C' as u32, ALT),
    (MPR_TOGGLE_CENTER_ALL, "mpr_toggle_center_all", "mpr", Mpr, 'C' as u32, CTRL | ALT),
    (MPR_TOGGLE_CROSS_LINES, "mpr_toggle_cross", "mpr", Mpr, 'V' as u32, ALT),
    (MPR_TOGGLE_CROSS_LINES_ALL, "mpr_toggle_cross_all", "mpr", Mpr, 'V' as u32, CTRL | ALT),
    (MPR_CYCLE_MIP, "mpr_cycle_mip", "mpr", Mpr, 'B' as u32, CTRL | ALT),
    (DOCKING_MAXIMIZE, "dock_maximize", "docking", ViewCanvas, 'M' as u32, CTRL),
    (DOCKING_EXTERNALIZE, "dock_externalize", "docking", ViewCanvas, 'E' as u32, CTRL),
    (DOCKING_NORMALIZE, "dock_normalize", "docking", ViewCanvas, 'N' as u32, CTRL),
    (DOCKING_CLOSE, "dock_close", "docking", ViewCanvas, 'W' as u32, CTRL),
    (DOCKING_PANEL_LIST, "dock_panel_list", "docking", ViewCanvas, 'E' as u32, CTRL | SHIFT),
    (DOCKING_NEXT_TAB, "dock_next_tab", "docking", ViewCanvas, key::TAB, CTRL),
    (DOCKING_PREV_TAB, "dock_prev_tab", "docking", ViewCanvas, key::TAB, CTRL | SHIFT),
    (EXPLORER_OPEN, "explorer_open", "dicom_explorer", DicomExplorer, key::ENTER, 0),
    (EXPLORER_SELECT_NEXT, "explorer_next", "dicom_explorer", DicomExplorer, key::DOWN, 0),
    (EXPLORER_SELECT_PREVIOUS, "explorer_prev", "dicom_explorer", DicomExplorer, key::UP, 0),
    (EXPLORER_SELECT_FIRST, "explorer_first", "dicom_explorer", DicomExplorer, key::HOME, 0),
    (EXPLORER_SELECT_FIRST_ALT, "explorer_first", "dicom_explorer", DicomExplorer, key::PAGE_UP, 0),
    (EXPLORER_SELECT_LAST, "explorer_last", "dicom_explorer", DicomExplorer, key::END, 0),
    (EXPLORER_SELECT_LAST_ALT, "explorer_last", "dicom_explorer", DicomExplorer, key::PAGE_DOWN, 0),
    (EXPLORER_SELECT_ALL, "explorer_select_all", "dicom_explorer", DicomExplorer, 'A' as u32, CTRL),
];

impl ShortcutManager {
    /// The shortcuts of the application, the menu ones with Cmd on macOS and Ctrl elsewhere.
    // TODO(3.7): also the keys of the ActionW mouse actions (`registerActionWFeatures`).
    pub fn with_defaults(is_mac: bool) -> Self {
        let mut manager = Self::default();
        for (id, description, category, context, key_code, modifiers) in DEFAULTS {
            let modifiers = if modifiers & MENU == 0 {
                modifiers
            } else {
                modifiers & !MENU | modifier::menu_shortcut(is_mac)
            };
            let description = format!("ShortcutManager.{description}");
            let category = format!("ShortcutManager.cat.{category}");
            let binding = KeyBinding::new(key_code, modifiers);
            manager.register(id, &description, &category, context, binding);
        }
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::key_binding::modifier::META;

    #[test]
    fn defaults_follow_the_desktop_application() {
        let manager = ShortcutManager::with_defaults(false);
        assert_eq!(manager.entries().len(), 77);
        assert_eq!(manager.binding(VIEWER_ZOOM_IN).text(), "Ctrl+NumPad +");
        assert_eq!(manager.binding(VIEWER_SCREENSHOT).text(), "Ctrl+Shift+S");
        let explorer = manager.entry(EXPLORER_SELECT_ALL).map(|e| e.context);
        assert_eq!(explorer, Some(DicomExplorer));
        let entry = manager.entry(DOCKING_NEXT_TAB);
        assert_eq!(
            entry.map(|e| e.category.as_str()),
            Some("ShortcutManager.cat.docking")
        );
    }

    #[test]
    fn menu_shortcuts_use_cmd_on_macos() {
        let manager = ShortcutManager::with_defaults(true);
        assert!(manager.matches(DRAW_COPY, 'C' as u32, META));
        assert!(manager.matches(VIEWER_SCREENSHOT, 'S' as u32, META | SHIFT));
        assert!(
            manager.matches(DICOM_NEXT_STUDY, key::RIGHT, CTRL),
            "not a menu shortcut"
        );
    }

    #[test]
    fn the_defaults_have_no_conflict_but_the_known_ones() {
        let manager = ShortcutManager::with_defaults(false);
        let mut conflicts: Vec<(String, String)> = Vec::new();
        for entry in manager.entries() {
            for other in manager.find_conflicts(&entry.id, entry.binding, entry.context) {
                if entry.id < other.id {
                    conflicts.push((entry.id.clone(), other.id.clone()));
                }
            }
        }
        // Escape is shared with the drawing in progress, which gets the key first.
        let escape = ("draw.cancelDrawing".to_owned(), "viewer.escape".to_owned());
        assert_eq!(conflicts, [escape]);
    }
}
