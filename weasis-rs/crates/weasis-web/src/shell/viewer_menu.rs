//! Context menu of a viewer tab: close the others or all the tabs of its group, split the group,
//! move the tab into another group.
//!
//! Java reference: `ViewerPlugin.CloseOthersAction` and
//! `weasis-core/src/main/java/org/weasis/core/ui/editor/image/TabPlacement.java`.

use leptos::prelude::*;

use super::tab_stack::TabId;
use super::viewers::Viewers;
use super::working_area::{NodeId, WorkingArea};
use crate::i18n::tr;

/// Longest group name in the Move to Other Group submenu.
const MAX_MENU_LABEL_LENGTH: usize = 60;

/// Tab whose menu is open, and where.
#[derive(Clone, Copy, PartialEq)]
pub struct TabMenuAt {
    /// The tab.
    pub tab: TabId,
    /// Pointer position in the page.
    pub x: i32,
    /// Pointer position in the page.
    pub y: i32,
}

/// The menu, shown while `at` is set.
#[component]
pub fn TabMenu(viewers: Viewers, at: RwSignal<Option<TabMenuAt>>) -> impl IntoView {
    let area = viewers.area;
    move || {
        at.get().map(|TabMenuAt { tab, x, y }| {
            let dismiss = move || at.set(None);
            let close = move |change: fn(&mut WorkingArea, TabId) -> Vec<TabId>| {
                dismiss();
                let removed = area.try_update(|a| change(a, tab)).unwrap_or_default();
                viewers.forget(removed);
            };
            let split = move |down| {
                dismiss();
                area.update(|a| a.split(tab, down));
            };
            let can_split = area.with_untracked(|a| a.can_split(tab));
            let groups = other_groups(viewers, tab)
                .into_iter()
                .map(|(group, label)| {
                    view! {
                        <li role="menuitem" class="w-menu-item" on:click=move |_| {
                            dismiss();
                            area.update(|a| a.move_to(tab, group));
                        }>
                            {label}
                        </li>
                    }
                })
                .collect::<Vec<_>>();
            let no_group = groups.is_empty();
            view! {
                <div class="w-backdrop" on:click=move |_| dismiss()></div>
                <ul class="w-menu w-context-menu" role="menu" style=format!("left: {x}px; top: {y}px")>
                    <li role="menuitem" class="w-menu-item" on:click=move |_| close(WorkingArea::close_others)>
                        {tr("ViewerPlugin.close_other")}
                    </li>
                    <li role="menuitem" class="w-menu-item" on:click=move |_| close(WorkingArea::close_group)>
                        {tr("ViewerPlugin.close_all")}
                    </li>
                    <li role="separator" class="w-menu-separator"></li>
                    <li
                        role="menuitem"
                        class="w-menu-item"
                        title=tr("ViewerPlugin.split_right_tip")
                        aria-disabled=(!can_split).to_string()
                        on:click=move |_| split(false)
                    >
                        {tr("ViewerPlugin.split_right")}
                    </li>
                    <li
                        role="menuitem"
                        class="w-menu-item"
                        title=tr("ViewerPlugin.split_down_tip")
                        aria-disabled=(!can_split).to_string()
                        on:click=move |_| split(true)
                    >
                        {tr("ViewerPlugin.split_down")}
                    </li>
                    <li
                        role="menuitem"
                        class="w-menu-item w-submenu"
                        aria-haspopup="menu"
                        title=tr("ViewerPlugin.move_group_tip")
                        aria-disabled=no_group.to_string()
                    >
                        <span>{tr("ViewerPlugin.move_group")}</span>
                        <span class="w-submenu-arrow" aria-hidden="true"></span>
                        <ul class="w-menu" role="menu">{groups}</ul>
                    </li>
                </ul>
            }
        })
    }
}

/// The groups other than the one of `tab`, named after the titles of their viewers.
fn other_groups(viewers: Viewers, tab: TabId) -> Vec<(NodeId, String)> {
    viewers.area.with_untracked(|a| {
        let current = a.group_of(tab);
        a.groups()
            .into_iter()
            .filter(|&g| Some(g) != current)
            .filter_map(|g| {
                let mut titles: Vec<String> = Vec::new();
                for t in a.group(g)?.tabs() {
                    let title = viewers.title(*t);
                    if !titles.contains(&title) {
                        titles.push(title);
                    }
                }
                Some((g, menu_label(&titles.join(", "))))
            })
            .collect()
    })
}

fn menu_label(text: &str) -> String {
    if text.chars().count() > MAX_MENU_LABEL_LENGTH {
        let cut: String = text.chars().take(MAX_MENU_LABEL_LENGTH).collect();
        format!("{cut}…")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_group_names_are_cut() {
        assert_eq!(menu_label("short"), "short");
        let long = "x".repeat(61);
        assert_eq!(menu_label(&long), format!("{}…", "x".repeat(60)));
    }
}
