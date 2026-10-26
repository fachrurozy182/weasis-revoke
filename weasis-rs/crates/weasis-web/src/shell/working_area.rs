//! The viewer area as a tree of tab groups: a group is split in two side by side or one above the
//! other, a tab is moved into another group, and a group left empty disappears with its split.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/editor/image/TabPlacement.java`
//! (the DockingFrames `CWorkingArea`: a `SplitDockStation` of `StackDockStation`s).

use std::collections::BTreeMap;

use super::tab_stack::{TabId, TabStack};

/// Identifier of a node of the tree (a tab group or a split).
pub type NodeId = u32;

/// Smallest and largest share of the first part of a split.
const RATIO_RANGE: (f64, f64) = (0.1, 0.9);

/// A node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Tabs shown in the same place.
    Group(TabStack),
    /// Two parts side by side (`down` false) or one above the other.
    Split {
        /// The second part is below the first.
        down: bool,
        /// Share of the first part.
        ratio: f64,
        /// Left or top part.
        first: NodeId,
        /// Right or bottom part.
        second: NodeId,
    },
}

/// The tab groups of the viewer area.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkingArea {
    nodes: BTreeMap<NodeId, Node>,
    root: Option<NodeId>,
    /// Group of the last selected tab, where new tabs open.
    focused: Option<NodeId>,
    next_id: NodeId,
}

impl WorkingArea {
    /// The root node.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// A node.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The tabs of a group.
    pub fn group(&self, id: NodeId) -> Option<&TabStack> {
        match self.nodes.get(&id) {
            Some(Node::Group(stack)) => Some(stack),
            _ => None,
        }
    }

    /// The groups, by id.
    pub fn groups(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| matches!(node, Node::Group(_)))
            .map(|(id, _)| *id)
            .collect()
    }

    /// The group holding a tab.
    pub fn group_of(&self, tab: TabId) -> Option<NodeId> {
        self.groups()
            .into_iter()
            .find(|&g| self.group(g).is_some_and(|s| s.tabs().contains(&tab)))
    }

    /// The selected tab of the focused group.
    pub fn selected(&self) -> Option<TabId> {
        self.focused
            .and_then(|g| self.group(g))
            .and_then(TabStack::selected)
    }

    /// Opens a tab in the focused group, or in the first group when none has the focus.
    pub fn open(&mut self, tab: TabId) {
        let group = match self.focused.or_else(|| self.groups().first().copied()) {
            Some(group) => group,
            None => {
                let group = self.add(Node::Group(TabStack::default()));
                self.root = Some(group);
                group
            }
        };
        if let Some(stack) = self.stack_mut(group) {
            stack.open(tab);
            self.focused = Some(group);
        }
    }

    /// Selects a tab and focuses its group.
    pub fn select(&mut self, tab: TabId) {
        if let Some(group) = self.group_of(tab) {
            self.with_stack(group, |s| s.select(tab));
            self.focused = Some(group);
        }
    }

    /// Selects the next (or previous) tab of the focused group, going round.
    pub fn select_next(&mut self, forward: bool) {
        let Some(stack) = self.focused.and_then(|g| self.group(g)) else {
            return;
        };
        let tabs = stack.tabs();
        let Some(index) = stack
            .selected()
            .and_then(|t| tabs.iter().position(|&x| x == t))
        else {
            return;
        };
        let next = if forward {
            (index + 1) % tabs.len()
        } else {
            (index + tabs.len() - 1) % tabs.len()
        };
        let tab = tabs[next];
        self.select(tab);
    }

    /// Removes a tab.
    pub fn close(&mut self, tab: TabId) {
        if let Some(group) = self.group_of(tab) {
            self.with_stack(group, |s| s.close(tab));
            self.prune(group);
        }
    }

    /// Removes the other tabs of the group of `tab`; returns them.
    pub fn close_others(&mut self, tab: TabId) -> Vec<TabId> {
        let Some(group) = self.group_of(tab) else {
            return Vec::new();
        };
        self.focused = Some(group);
        self.with_stack(group, |s| s.close_others(tab))
    }

    /// Removes every tab of the group of `tab`; returns them.
    pub fn close_group(&mut self, tab: TabId) -> Vec<TabId> {
        let Some(group) = self.group_of(tab) else {
            return Vec::new();
        };
        let removed = self.with_stack(group, TabStack::close_all);
        self.prune(group);
        removed
    }

    /// Whether the group of `tab` can be split, i.e. holds other tabs.
    pub fn can_split(&self, tab: TabId) -> bool {
        self.group_of(tab)
            .and_then(|g| self.group(g))
            .is_some_and(|s| s.tabs().len() > 1)
    }

    /// Moves `tab` into a new group taking the right (or bottom) half of its group.
    pub fn split(&mut self, tab: TabId, down: bool) {
        let Some(group) = self.group_of(tab).filter(|_| self.can_split(tab)) else {
            return;
        };
        let Some(Node::Group(mut old)) = self.nodes.remove(&group) else {
            return;
        };
        old.close(tab);
        let mut stack = TabStack::default();
        stack.open(tab);
        // The group becomes a split of its old content and the new group, in the same place.
        let old_content = self.add(Node::Group(old));
        let new_group = self.add(Node::Group(stack));
        self.nodes.insert(
            group,
            Node::Split {
                down,
                ratio: 0.5,
                first: old_content,
                second: new_group,
            },
        );
        self.focused = Some(new_group);
    }

    /// Moves `tab` into the group `target`.
    pub fn move_to(&mut self, tab: TabId, target: NodeId) {
        let Some(from) = self.group_of(tab).filter(|&g| g != target) else {
            return;
        };
        if self.group(target).is_none() {
            return;
        }
        self.with_stack(from, |s| s.close(tab));
        self.with_stack(target, |s| s.open(tab));
        self.focused = Some(target);
        self.prune(from);
    }

    /// Sets the share of the first part of a split.
    pub fn resize(&mut self, split: NodeId, share: f64) {
        if let Some(Node::Split { ratio, .. }) = self.nodes.get_mut(&split) {
            *ratio = share.clamp(RATIO_RANGE.0, RATIO_RANGE.1);
        }
    }

    fn add(&mut self, node: Node) -> NodeId {
        self.next_id += 1;
        self.nodes.insert(self.next_id, node);
        self.next_id
    }

    fn stack_mut(&mut self, group: NodeId) -> Option<&mut TabStack> {
        match self.nodes.get_mut(&group) {
            Some(Node::Group(stack)) => Some(stack),
            _ => None,
        }
    }

    /// Runs `change` on the tabs of a group; the default result when `group` is not a group.
    fn with_stack<R: Default>(
        &mut self,
        group: NodeId,
        change: impl FnOnce(&mut TabStack) -> R,
    ) -> R {
        self.stack_mut(group).map(change).unwrap_or_default()
    }

    /// Removes a group left empty: its sibling takes the place of their split. The last group
    /// stays, empty, like the working area.
    fn prune(&mut self, group: NodeId) {
        if self.group(group).is_none_or(|s| !s.tabs().is_empty()) {
            return;
        }
        let parent = self.nodes.iter().find_map(|(&id, node)| match node {
            Node::Split { first, second, .. } if *first == group => Some((id, *second)),
            Node::Split { first, second, .. } if *second == group => Some((id, *first)),
            _ => None,
        });
        let Some((split, sibling)) = parent else {
            return;
        };
        self.nodes.remove(&group);
        // The sibling moves into the id of the split, so that the grandparent needs no change.
        if let Some(node) = self.nodes.remove(&sibling) {
            self.nodes.insert(split, node);
        }
        if self.focused.is_none_or(|f| f == group || f == sibling) {
            self.focused = self.first_group(split);
        }
    }

    fn first_group(&self, id: NodeId) -> Option<NodeId> {
        match self.nodes.get(&id)? {
            Node::Group(_) => Some(id),
            Node::Split { first, .. } => self.first_group(*first),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> WorkingArea {
        let mut area = WorkingArea::default();
        (1..=3).for_each(|tab| area.open(tab));
        area
    }

    fn tabs(area: &WorkingArea, group: NodeId) -> Vec<TabId> {
        area.group(group)
            .map(|s| s.tabs().to_vec())
            .unwrap_or_default()
    }

    #[test]
    fn tabs_open_in_one_group() {
        let area = area();
        assert_eq!(area.groups().len(), 1);
        assert_eq!(tabs(&area, area.groups()[0]), [1, 2, 3]);
        assert_eq!(area.selected(), Some(3));
    }

    #[test]
    fn a_split_puts_the_tab_in_a_new_half_and_focuses_it() {
        let mut area = area();
        area.split(2, false);
        let root = area.root().expect("root");
        let Some(Node::Split {
            down,
            first,
            second,
            ..
        }) = area.node(root).cloned()
        else {
            panic!("root is a split");
        };
        assert!(!down);
        assert_eq!(
            (tabs(&area, first), tabs(&area, second)),
            (vec![1, 3], vec![2])
        );
        assert_eq!(area.selected(), Some(2));
        area.open(4);
        assert_eq!(
            tabs(&area, second),
            [2, 4],
            "new tabs go to the focused group"
        );
        let mut alone = WorkingArea::default();
        alone.open(1);
        alone.split(1, true);
        assert_eq!(alone.groups().len(), 1, "a single tab cannot split");
    }

    #[test]
    fn an_emptied_group_gives_its_place_to_its_sibling() {
        let mut area = area();
        area.split(3, false);
        area.split(1, true);
        assert_eq!(area.groups().len(), 3);
        area.close(3);
        assert_eq!(area.groups().len(), 2);
        let first = area.groups()[0];
        area.move_to(1, first);
        assert_eq!(area.groups().len(), 1);
        let root = area.root().expect("root");
        assert_eq!(tabs(&area, root), [2, 1]);
        assert_eq!(area.selected(), Some(1));
    }

    #[test]
    fn next_and_previous_tabs_go_round_in_the_focused_group() {
        let mut area = area();
        area.select_next(true);
        assert_eq!(area.selected(), Some(1));
        area.select_next(false);
        assert_eq!(area.selected(), Some(3));
        area.split(3, false);
        area.select_next(true);
        assert_eq!(area.selected(), Some(3), "alone in its group");
    }

    #[test]
    fn closing_the_last_tab_keeps_an_empty_area_and_ratios_stay_in_range() {
        let mut area = area();
        area.split(1, false);
        let root = area.root().expect("root");
        area.resize(root, 0.99);
        assert!(matches!(area.node(root), Some(Node::Split { ratio, .. }) if *ratio == 0.9));
        assert_eq!(area.close_group(1), [1]);
        assert_eq!(area.close_others(2), [3]);
        area.close(2);
        assert_eq!(area.groups().len(), 1);
        assert_eq!(area.selected(), None);
        area.open(5);
        assert_eq!(area.selected(), Some(5));
    }
}
