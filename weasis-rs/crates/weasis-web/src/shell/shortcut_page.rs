//! Help ▸ Shortcuts: a standalone page listing the current bindings by category, then the
//! shortcuts that cannot be changed (presets, mouse gestures), opened in a new browser tab like
//! the desktop application opens it in the default browser.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/ShortcutManager.java`
//! (`generateHtmlPage`, `appendFixedShortcutsSection`, `buildCategoryDocTopics`).

use weasis_core::gui::{ShortcutEntry, ShortcutManager};

use crate::help::online_help_url;
use crate::i18n::tr;

const STYLE: &str = include_str!("../../style/shortcut-page.css");

/// A section of shortcuts that cannot be changed: title key, tutorial topic, then rows of
/// (shortcut key, action key).
type FixedSection = (
    &'static str,
    Option<&'static str>,
    &'static [(&'static str, &'static str)],
);

const M: &str = "ShortcutManager.html.mouse.";

const FIXED_SECTIONS: [FixedSection; 6] = [
    (
        "ShortcutManager.html.dicom_2d_viewer",
        Some("dicom-2d-viewer"),
        &[
            ("left_drag", "left_drag_action"),
            ("middle_drag", "middle_drag_action"),
            ("right_click", "right_click_action"),
            ("scroll", "scroll_action"),
            ("ctrl_drag", "ctrl_drag_action"),
            ("ctrl_shift_drag", "ctrl_shift_drag_action"),
            ("dblclick_f11", "dblclick_f11_action"),
        ],
    ),
    (
        "ShortcutManager.html.mpr_viewer",
        Some("mpr"),
        &[("alt_scroll", "alt_scroll_action")],
    ),
    (
        "ShortcutManager.html.graphics",
        Some("draw-measure"),
        &[
            ("click_graphic", "click_graphic_action"),
            ("click_drag", "click_drag_action"),
            ("shift_click_graphic", "shift_click_graphic_action"),
            ("dblclick", "dblclick_action"),
        ],
    ),
    (
        "ShortcutManager.ctx.dicom_explorer",
        Some("dicom-explorer"),
        &[
            ("ctrl_click_thumb", "ctrl_click_thumb_action"),
            ("shift_click_thumb", "shift_click_thumb_action"),
            ("right_click", "right_click_explorer_action"),
        ],
    ),
    (
        "ShortcutManager.html.drag_drop",
        None,
        &[
            ("drag_files", "drag_files_action"),
            ("drag_thumb", "drag_thumb_action"),
        ],
    ),
    (
        "ShortcutManager.html.tabs",
        None,
        &[("tab_right_click", "tab_right_click_action")],
    ),
];

/// The page, as an HTML document.
pub fn shortcut_page(shortcuts: &ShortcutManager) -> String {
    let mut html = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
         <title>{title}</title>\n<style>\n{STYLE}</style>\n</head>\n<body>\n\
         <h1>⌨ {title}</h1>\n<div class=\"subtitle\">{subtitle}</div>\n\
         <div class=\"doc-bar\"><a href=\"{doc}\" target=\"_blank\">↗ {online}</a></div>\n",
        title = text("ShortcutManager.html.title"),
        subtitle = text("ShortcutManager.html.subtitle"),
        doc = escape(&online_help_url("index.html")),
        online = text("ShortcutManager.html.online_doc"),
    );
    for (category, entries) in by_category(shortcuts.entries()) {
        html.push_str(&category_section(&category, &entries));
    }
    html.push_str(&fixed_sections());
    html.push_str(&format!(
        "<footer>{} · Weasis {}</footer>\n</body>\n</html>\n",
        text("ShortcutManager.html.footer"),
        env!("CARGO_PKG_VERSION")
    ));
    html
}

/// Entries sorted by category, context and description names, grouped by category.
fn by_category(entries: &[ShortcutEntry]) -> Vec<(String, Vec<&ShortcutEntry>)> {
    let mut sorted: Vec<&ShortcutEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| {
        (
            tr(&e.category),
            tr(e.context.name_key()),
            tr(&e.description),
        )
    });
    let mut groups: Vec<(String, Vec<&ShortcutEntry>)> = Vec::new();
    for entry in sorted {
        match groups.last_mut() {
            Some((category, list)) if *category == entry.category => list.push(entry),
            _ => groups.push((entry.category.clone(), vec![entry])),
        }
    }
    groups
}

fn category_section(category: &str, entries: &[&ShortcutEntry]) -> String {
    let mut html = format!(
        "<div class=\"category\">\n<h2>{}{}</h2>\n<table>\n",
        text(category),
        doc_link(category_topic(category))
    );
    html.push_str(&header(&[
        "ShortcutManager.html.col.action",
        "ShortcutManager.html.col.context",
        "ShortcutManager.html.col.shortcut",
        "ShortcutManager.html.col.default",
    ]));
    for entry in entries {
        let shortcut = match entry.binding.text() {
            t if t.is_empty() => format!(
                "<em style=\"color:var(--subtext)\">{}</em>",
                text("ShortcutManager.html.none")
            ),
            t => kbd(&t),
        };
        let default = match entry.default_binding.text() {
            t if entry.is_modified() && !t.is_empty() => kbd(&t),
            _ => "—".to_owned(),
        };
        html.push_str(&format!(
            "<tr{}><td>{}</td><td>{}</td><td>{shortcut}</td><td>{default}</td></tr>\n",
            if entry.is_modified() {
                " class=\"modified\""
            } else {
                ""
            },
            text(&entry.description),
            text(entry.context.name_key()),
        ));
    }
    html.push_str("</table>\n</div>\n");
    html
}

fn fixed_sections() -> String {
    let columns = [
        "ShortcutManager.html.col.shortcut",
        "ShortcutManager.html.col.action",
    ];
    let mut html = format!(
        "<div class=\"fixed-section\">\n<h2>🔒 {}</h2>\n<h3>{}</h3>\n\
         <div class=\"info-note\">{}</div>\n<table>\n{}\
         <tr><td>{}</td><td>{}</td><td>{}</td></tr>\n</table>\n</div>\n",
        text("ShortcutManager.html.fixed_title"),
        text("ShortcutManager.html.dicom_presets"),
        text("ShortcutManager.html.dicom_presets_note"),
        header(&[columns[0], columns[1], "ShortcutManager.html.col.note"]),
        kbd("0 1 2 3 … 9"),
        text("ShortcutManager.html.dicom_presets_action"),
        text("ShortcutManager.html.dicom_presets_note_col"),
    );
    html.push_str(&format!(
        "<div class=\"fixed-section\">\n<h2>🖱 {}</h2>\n<div class=\"info-note\">{}</div>\n",
        text("ShortcutManager.html.mouse_title"),
        text("ShortcutManager.html.mouse_note")
    ));
    for (title, topic, rows) in FIXED_SECTIONS {
        html.push_str(&format!(
            "<h3>{}{}</h3>\n<table>\n",
            text(title),
            doc_link(topic)
        ));
        html.push_str(&header(&columns));
        for (shortcut, action) in rows {
            let shortcut = text(&format!("{M}{shortcut}"));
            let action = text(&format!("{M}{action}"));
            html.push_str(&format!(
                "<tr><td><span class=\"kbd\">{shortcut}</span></td><td>{action}</td></tr>\n"
            ));
        }
        html.push_str("</table>\n");
    }
    html.push_str("</div>\n");
    html
}

/// Tutorial page of a category, for the categories that have one.
fn category_topic(category: &str) -> Option<&'static str> {
    let name = category.strip_prefix("ShortcutManager.cat.")?;
    match name {
        "mouse_actions" | "viewer" | "navigation" | "display" | "dicom_nav" | "pan" => {
            Some("dicom-2d-viewer")
        }
        "measures" | "drawings" => Some("draw-measure"),
        "mpr" => Some("mpr"),
        "dicom_explorer" => Some("dicom-explorer"),
        _ => None,
    }
}

fn header(keys: &[&str]) -> String {
    let cells: String = keys
        .iter()
        .map(|k| format!("<th>{}</th>", text(k)))
        .collect();
    format!("<tr>{cells}</tr>\n")
}

fn doc_link(topic: Option<&str>) -> String {
    topic.map_or_else(String::new, |topic| {
        format!(
            " <a class=\"doc-link\" href=\"{}\" target=\"_blank\">↗</a>",
            escape(&online_help_url(topic))
        )
    })
}

fn kbd(shortcut: &str) -> String {
    format!("<span class=\"kbd\">{}</span>", escape(shortcut))
}

/// Translated and escaped text of a message key.
fn text(key: &str) -> String {
    escape(tr(key))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use weasis_core::gui::{KeyBinding, id, modifier};

    #[test]
    fn the_page_lists_the_bindings_by_category() {
        let mut shortcuts = ShortcutManager::with_defaults(false);
        let ctrl_k = KeyBinding::new('K' as u32, modifier::CTRL);
        shortcuts.set_shortcut(id::VIEWER_ZOOM_IN, ctrl_k);
        let html = shortcut_page(&shortcuts);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<h2>Pan <a class=\"doc-link\""));
        let zoom_in = "<tr class=\"modified\"><td>Zoom in</td><td>View Canvas</td>\
                       <td><span class=\"kbd\">Ctrl+K</span></td>\
                       <td><span class=\"kbd\">Ctrl+NumPad +</span></td></tr>";
        assert!(html.contains(zoom_in), "modified row with its default");
        assert!(html.contains("<td>Zoom out</td><td>View Canvas</td><td><span class=\"kbd\">Ctrl+NumPad -</span></td><td>—</td>"));
        assert!(html.contains("Alt + mouse scroll"));
        assert_eq!(escape("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }
}
