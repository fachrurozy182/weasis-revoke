//! The Weasis look: the FlatLaf theme of the desktop application turned into CSS custom
//! properties.
//!
//! The theme is an IntelliJ theme document: named `colors`, and `ui` keys per component whose
//! values are color names, `#RRGGBB[AA]` colors, sizes or insets. Every key becomes a variable,
//! e.g. `Button.arc` → `--Button-arc: 8px` and `*.background` → `--ui-background`; the style
//! sheet reads them with a fallback to the `*` defaults.
//!
//! Java reference: `weasis-launcher/src/main/java/org/weasis/launcher/FlatWeasisTheme.java`,
//! `weasis-launcher/src/main/resources/org/weasis/theme/Weasis.theme.json` and FlatLaf
//! `IntelliJTheme`.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// The theme of the desktop application, reused as it is (decision D7).
pub const WEASIS_THEME_JSON: &str = include_str!("../../../assets/config/Weasis.theme.json");

/// A parsed theme.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Theme name.
    pub name: String,
    /// Whether the theme is dark.
    pub dark: bool,
    /// CSS variables (name without `--`, value), sorted by name.
    pub variables: BTreeMap<String, String>,
}

impl Theme {
    /// Parses an IntelliJ theme document.
    pub fn parse(json: &str) -> Result<Self, String> {
        let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let empty = Map::new();
        let colors = root
            .get("colors")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let mut variables = BTreeMap::new();
        for (name, value) in colors {
            if let Some(color) = value.as_str().and_then(css_color) {
                variables.insert(format!("color-{name}"), color);
            }
        }
        if let Some(ui) = root.get("ui").and_then(Value::as_object) {
            flatten(ui, "", &mut |key, value| {
                if let Some(css) = css_value(key, value, colors) {
                    variables.insert(variable_name(key), css);
                }
            });
        }
        Ok(Self {
            name: root
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Weasis")
                .to_owned(),
            dark: root.get("dark").and_then(Value::as_bool).unwrap_or(true),
            variables,
        })
    }

    /// The Weasis theme.
    pub fn weasis() -> Result<Self, String> {
        Self::parse(WEASIS_THEME_JSON)
    }

    /// Style sheet declaring the variables on `:root`.
    pub fn css(&self) -> String {
        let mut css = String::from(":root {\n");
        css.push_str(&format!(
            "  color-scheme: {};\n",
            if self.dark { "dark" } else { "light" }
        ));
        for (name, value) in &self.variables {
            css.push_str(&format!("  --{name}: {value};\n"));
        }
        css.push_str("}\n");
        css
    }
}

/// Calls `visit` for each leaf of the `ui` object, with its dotted key.
fn flatten(object: &Map<String, Value>, prefix: &str, visit: &mut impl FnMut(&str, &Value)) {
    for (key, value) in object {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Object(child) => flatten(child, &path, visit),
            leaf => visit(&path, leaf),
        }
    }
}

/// `*.background` → `ui-background`, `Button.default.background` → `Button-default-background`.
fn variable_name(key: &str) -> String {
    let name = match key.strip_prefix("*.") {
        Some(rest) => format!("ui-{rest}"),
        None => key.to_owned(),
    };
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// CSS value of a theme value: colors (named or hexadecimal), sizes in px, insets
/// (`top,left,bottom,right`) as CSS padding (`top right bottom left`). Other values are left out.
fn css_value(key: &str, value: &Value, colors: &Map<String, Value>) -> Option<String> {
    match value {
        Value::String(text) => {
            let named = colors.get(text).and_then(Value::as_str);
            named
                .or(Some(text.as_str()))
                .and_then(css_color)
                .or_else(|| insets(key, text))
        }
        Value::Number(number) => {
            let unitless = ["alpha", "Alpha", "opacity"]
                .iter()
                .any(|word| key.contains(word));
            let number = number.as_f64()?;
            Some(if unitless {
                format!("{number}")
            } else {
                format!("{number}px")
            })
        }
        _ => None,
    }
}

/// `#RGB`, `#RRGGBB` or `#RRGGBBAA` (IntelliJ puts the alpha last, like CSS).
fn css_color(text: &str) -> Option<String> {
    let hex = text.strip_prefix('#')?;
    (matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{hex}"))
}

/// Insets `top,left,bottom,right` as CSS `top right bottom left`, for keys naming insets.
fn insets(key: &str, text: &str) -> Option<String> {
    let lower = key.to_lowercase();
    if !(lower.contains("insets") || lower.contains("padding") || lower.contains("margin")) {
        return None;
    }
    let values: Vec<i32> = text
        .split(',')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<_>>()?;
    let [top, left, bottom, right] = values[..] else {
        return None;
    };
    Some(format!("{top}px {right}px {bottom}px {left}px"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weasis_theme_gives_its_variables() {
        let theme = Theme::weasis().expect("valid theme");
        assert_eq!(theme.name, "Monokai Pro Contrast");
        assert!(theme.dark);
        let var = |name: &str| theme.variables.get(name).map(String::as_str);
        assert_eq!(var("color-bg"), Some("#2D2A2E"));
        assert_eq!(var("ui-background"), Some("#2D2A2E"), "* defaults");
        assert_eq!(var("ui-focusedBorderColor"), Some("#ffd866"));
        assert_eq!(var("Button-arc"), Some("8px"));
        assert_eq!(var("ComboBox-padding"), Some("5px 5px 5px 5px"));
        assert!(theme.css().starts_with(":root {\n  color-scheme: dark;\n"));
    }

    #[test]
    fn values_become_css() {
        let theme = Theme::parse(
            r##"{"name": "T", "dark": false, "colors": {"accent": "#ff000080"},
                "ui": {"*": {"foreground": "accent"},
                       "Panel": {"insets": "1,2,3,4", "arc": 5, "iconAlpha": 0.5,
                                 "unknown": "noColor", "flag": true}}}"##,
        )
        .expect("valid");
        let var = |name: &str| theme.variables.get(name).map(String::as_str);
        assert_eq!(var("ui-foreground"), Some("#ff000080"));
        assert_eq!(var("Panel-insets"), Some("1px 4px 3px 2px"));
        assert_eq!(var("Panel-arc"), Some("5px"));
        assert_eq!(var("Panel-iconAlpha"), Some("0.5"));
        assert_eq!(var("Panel-unknown"), None);
        assert_eq!(var("Panel-flag"), None);
        assert!(theme.css().contains("color-scheme: light"));
    }
}
