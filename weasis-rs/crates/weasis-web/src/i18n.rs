//! Texts of the interface, read from the message bundles of the Java modules (keys unchanged,
//! e.g. `WeasisWin.file`). Only English for now; the translations come in step 2.9.
//!
//! Java reference: the `Messages.java` + `messages.properties` pair of each module.

use std::collections::HashMap;
use std::sync::OnceLock;

/// English bundles, in lookup order.
const BUNDLES: [&str; 2] = [
    include_str!("../../../assets/i18n/base-ui.properties"),
    include_str!("../../../assets/i18n/core.properties"),
];

/// Text of a key; the key itself when it is missing (like the Java `Messages.getString`, which
/// shows `!key!`).
pub fn tr(key: &str) -> &str {
    static TEXTS: OnceLock<HashMap<String, String>> = OnceLock::new();
    let texts = TEXTS.get_or_init(|| {
        let mut texts = HashMap::new();
        // The first bundle that defines a key wins.
        for bundle in BUNDLES.iter().rev() {
            texts.extend(parse_properties(bundle));
        }
        texts
    });
    texts.get(key).map_or(key, String::as_str)
}

/// Entries of a `.properties` file: `key=value` or `key: value`, `#` and `!` comments, lines
/// continued by a trailing backslash, and the `\n`, `\t`, `\uXXXX` escapes.
pub fn parse_properties(text: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut logical = String::new();
    for line in text.lines() {
        let line = line.trim_start();
        if logical.is_empty() && (line.is_empty() || line.starts_with('#') || line.starts_with('!'))
        {
            continue;
        }
        let continued = line.ends_with('\\') && !line.ends_with("\\\\");
        logical.push_str(if continued {
            &line[..line.len() - 1]
        } else {
            line
        });
        if continued {
            continue;
        }
        if let Some(index) = logical.find(['=', ':']) {
            let key = logical[..index].trim();
            let value = logical[index + 1..].trim_start();
            entries.push((unescape(key), unescape(value)));
        }
        logical.clear();
    }
    entries
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(decoded) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(decoded);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_of_the_java_bundles_are_found() {
        assert_eq!(tr("WeasisWin.file"), "File");
        assert_eq!(tr("OpenPreferencesAction.title"), "Preferences");
        assert_eq!(tr("no.such.key"), "no.such.key");
    }

    #[test]
    fn properties_syntax_is_supported() {
        let entries = parse_properties("# comment\na = one\nb: two \\\n  lines\nc=\\u00e9\\n\n");
        assert_eq!(
            entries,
            [
                ("a".to_owned(), "one".to_owned()),
                ("b".to_owned(), "two lines".to_owned()),
                ("c".to_owned(), "é\n".to_owned()),
            ]
        );
    }
}
