//! The series selected in the explorer: a click selects one series, Ctrl toggles one, Shift
//! extends from the anchor; the keyboard moves the selection through the series in the order
//! they are shown.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/SeriesSelectionModel.java`.

/// Keys of the selected series, in the order they were selected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeriesSelection {
    selected: Vec<String>,
    /// Where an interval starts.
    anchor: Option<String>,
    /// Where an interval ends, the series the keyboard moves from.
    lead: Option<String>,
}

impl SeriesSelection {
    /// Keys of the selected series.
    pub fn selected(&self) -> &[String] {
        &self.selected
    }

    /// Whether a series is selected.
    pub fn contains(&self, key: &str) -> bool {
        self.selected.iter().any(|s| s == key)
    }

    /// Adds a series, keeping the others (a right click on an unselected series).
    pub fn add(&mut self, key: &str) {
        if !self.contains(key) {
            self.selected.push(key.to_owned());
        }
    }

    /// Selects nothing.
    pub fn clear(&mut self) {
        self.selected.clear();
    }

    /// Selects exactly `keys`; the first one becomes the anchor and the last one the lead.
    pub fn set(&mut self, keys: Vec<String>) {
        self.anchor = keys.first().cloned();
        self.lead = keys.last().cloned();
        self.selected = keys;
    }

    /// A click on `key` with the modifiers held; `shown` is the order of the series on screen.
    pub fn click(&mut self, key: &str, shown: &[String], ctrl: bool, shift: bool) {
        match (ctrl, shift) {
            (true, true) => {
                let Some(anchor) = self.anchor.clone() else {
                    return;
                };
                let interval = interval(shown, &anchor, key);
                if self.contains(&anchor) {
                    self.add_all(interval);
                } else {
                    self.selected.retain(|s| !interval.contains(s));
                }
                self.lead = Some(key.to_owned());
            }
            (true, false) => {
                if self.contains(key) {
                    self.selected.retain(|s| s != key);
                } else {
                    self.selected.push(key.to_owned());
                }
                self.anchor = Some(key.to_owned());
                self.lead = Some(key.to_owned());
            }
            (false, true) => {
                if let Some(anchor) = self.anchor.clone() {
                    self.selected = interval(shown, &anchor, key);
                    self.lead = Some(key.to_owned());
                }
            }
            (false, false) => self.set(vec![key.to_owned()]),
        }
    }

    /// Selects every shown series.
    pub fn select_all(&mut self, shown: &[String]) {
        if let (Some(first), Some(last)) = (shown.first(), shown.last()) {
            let all = interval(shown, first, last);
            self.set(all);
        }
    }

    /// Selects the first shown series.
    pub fn select_first(&mut self, shown: &[String]) {
        self.set(shown.first().cloned().into_iter().collect());
    }

    /// Selects the last shown series.
    pub fn select_last(&mut self, shown: &[String]) {
        self.set(shown.last().cloned().into_iter().collect());
    }

    /// Selects the series after the lead, going round to the first.
    pub fn select_next(&mut self, shown: &[String]) {
        self.step(shown, true);
    }

    /// Selects the series before the lead, going round to the last.
    pub fn select_previous(&mut self, shown: &[String]) {
        self.step(shown, false);
    }

    /// Forgets the series that are no longer shown.
    pub fn retain_shown(&mut self, shown: &[String]) {
        self.selected.retain(|s| shown.contains(s));
    }

    fn step(&mut self, shown: &[String], forward: bool) {
        let Some(lead) = &self.lead else {
            return;
        };
        if shown.is_empty() {
            return;
        }
        let last = shown.len() - 1;
        let next = match (shown.iter().position(|s| s == lead), forward) {
            (Some(i), true) if i < last => i + 1,
            (_, true) => 0,
            (Some(i), false) if i > 0 => i - 1,
            (_, false) => last,
        };
        self.set(vec![shown[next].clone()]);
    }

    fn add_all(&mut self, keys: Vec<String>) {
        keys.iter().for_each(|key| self.add(key));
    }
}

/// The shown series from `a` to `b`, whichever comes first.
fn interval(shown: &[String], a: &str, b: &str) -> Vec<String> {
    let (Some(i), Some(j)) = (
        shown.iter().position(|s| s == a),
        shown.iter().position(|s| s == b),
    ) else {
        return Vec::new();
    };
    shown[i.min(j)..=i.max(j)].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown() -> Vec<String> {
        ["a", "b", "c", "d"].map(str::to_owned).to_vec()
    }

    #[test]
    fn clicks_select_toggle_and_extend() {
        let shown = shown();
        let mut selection = SeriesSelection::default();
        selection.click("b", &shown, false, false);
        assert_eq!(selection.selected(), ["b"]);
        selection.click("d", &shown, false, true);
        assert_eq!(selection.selected(), ["b", "c", "d"]);
        selection.click("c", &shown, true, false);
        assert_eq!(selection.selected(), ["b", "d"]);
        selection.click("a", &shown, true, true);
        // The anchor c is no longer selected: the interval a..c is removed.
        assert_eq!(selection.selected(), ["d"]);
        selection.click("a", &shown, true, false);
        selection.click("c", &shown, true, true);
        assert_eq!(selection.selected(), ["d", "a", "b", "c"]);
        selection.clear();
        selection.add("b");
        selection.add("b");
        assert_eq!(selection.selected(), ["b"]);
    }

    #[test]
    fn the_keyboard_moves_round_the_shown_series() {
        let shown = shown();
        let mut selection = SeriesSelection::default();
        selection.select_next(&shown);
        assert!(selection.selected().is_empty(), "nothing to move from");
        selection.select_last(&shown);
        selection.select_next(&shown);
        assert_eq!(selection.selected(), ["a"]);
        selection.select_previous(&shown);
        assert_eq!(selection.selected(), ["d"]);
        selection.select_all(&shown);
        assert_eq!(selection.selected().len(), 4);
        selection.retain_shown(&shown[..2]);
        assert_eq!(selection.selected(), ["a", "b"]);
    }
}
