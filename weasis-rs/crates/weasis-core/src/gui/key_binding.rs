//! A key with the keyboard modifiers held with it, in the AWT codes of the desktop application so
//! that shortcut preferences mean the same key in both.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/KeyBinding.java` and
//! the `VK_*` constants and `getKeyText` of `java.awt.event.KeyEvent`.

/// Keyboard modifiers: the extended masks of `java.awt.event.InputEvent`.
pub mod modifier {
    /// `SHIFT_DOWN_MASK`.
    pub const SHIFT: u32 = 1 << 6;
    /// `CTRL_DOWN_MASK`.
    pub const CTRL: u32 = 1 << 7;
    /// `META_DOWN_MASK` (Cmd on macOS).
    pub const META: u32 = 1 << 8;
    /// `ALT_DOWN_MASK`.
    pub const ALT: u32 = 1 << 9;
    /// `ALT_GRAPH_DOWN_MASK`.
    pub const ALT_GRAPH: u32 = 1 << 13;
    /// Every keyboard modifier; mouse button bits are outside.
    pub const KEYBOARD: u32 = SHIFT | CTRL | META | ALT | ALT_GRAPH;

    /// Modifier of the platform menu shortcuts: Cmd on macOS, Ctrl elsewhere.
    pub const fn menu_shortcut(is_mac: bool) -> u32 {
        if is_mac { META } else { CTRL }
    }
}

/// Key codes: the `VK_*` constants of `java.awt.event.KeyEvent`. Letters and digits are their
/// ASCII upper-case code (`'A' as u32`, `'0' as u32`) and F1 to F12 are `F1 + n - 1`.
pub mod key {
    #![allow(missing_docs)] // Named after the AWT constants.
    pub const BACK_SPACE: u32 = 8;
    pub const TAB: u32 = 9;
    pub const ENTER: u32 = 10;
    pub const ESCAPE: u32 = 27;
    pub const SPACE: u32 = 32;
    pub const PAGE_UP: u32 = 33;
    pub const PAGE_DOWN: u32 = 34;
    pub const END: u32 = 35;
    pub const HOME: u32 = 36;
    pub const LEFT: u32 = 37;
    pub const UP: u32 = 38;
    pub const RIGHT: u32 = 39;
    pub const DOWN: u32 = 40;
    pub const COMMA: u32 = 44;
    pub const MINUS: u32 = 45;
    pub const PERIOD: u32 = 46;
    pub const SLASH: u32 = 47;
    pub const SEMICOLON: u32 = 59;
    pub const EQUALS: u32 = 61;
    pub const OPEN_BRACKET: u32 = 91;
    pub const BACK_SLASH: u32 = 92;
    pub const CLOSE_BRACKET: u32 = 93;
    pub const NUMPAD0: u32 = 96;
    pub const MULTIPLY: u32 = 106;
    pub const ADD: u32 = 107;
    pub const SUBTRACT: u32 = 109;
    pub const DECIMAL: u32 = 110;
    pub const DIVIDE: u32 = 111;
    pub const F1: u32 = 112;
    pub const DELETE: u32 = 127;
    pub const INSERT: u32 = 155;
    pub const BACK_QUOTE: u32 = 192;
    pub const QUOTE: u32 = 222;
}

// Values of the deprecated InputEvent masks of the first AWT event model, found in older
// preferences: SHIFT_MASK, CTRL_MASK, META_MASK, ALT_MASK and ALT_GRAPH_MASK.
const LEGACY: [(u32, u32); 5] = [
    (1, modifier::SHIFT),
    (1 << 1, modifier::CTRL),
    (1 << 2, modifier::META),
    (1 << 3, modifier::ALT),
    (1 << 5, modifier::ALT_GRAPH),
];

/// A key code with its keyboard modifiers; key code 0 means unassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyBinding {
    /// AWT key code.
    pub key_code: u32,
    /// Extended keyboard modifiers.
    pub modifiers: u32,
}

impl KeyBinding {
    /// No key.
    pub const NONE: Self = Self {
        key_code: 0,
        modifiers: 0,
    };

    /// A binding; modifiers written with the legacy masks are converted and mouse bits dropped.
    pub fn new(key_code: u32, modifiers: u32) -> Self {
        Self {
            key_code,
            modifiers: to_extended(modifiers),
        }
    }

    /// Whether a key is assigned.
    pub fn is_assigned(&self) -> bool {
        self.key_code != 0
    }

    /// Same key and exactly the same keyboard modifiers.
    pub fn matches(&self, key_code: u32, modifiers: u32) -> bool {
        self.is_assigned() && self.key_code == key_code && self.modifiers == to_extended(modifiers)
    }

    /// Text such as "Ctrl+Shift+S", in the modifier order of the desktop application; empty when
    /// unassigned.
    pub fn text(&self) -> String {
        if !self.is_assigned() {
            return String::new();
        }
        let names = [
            (modifier::CTRL, "Ctrl+"),
            (modifier::ALT, "Alt+"),
            (modifier::ALT_GRAPH, "AltGr+"),
            (modifier::SHIFT, "Shift+"),
            (modifier::META, "Meta+"),
        ];
        let mut text: String = names
            .iter()
            .filter(|(mask, _)| self.modifiers & mask != 0)
            .map(|(_, name)| *name)
            .collect();
        text.push_str(&key_text(self.key_code));
        text
    }
}

/// Keyboard modifiers in their extended form, whatever form they were written in.
pub fn to_extended(modifiers: u32) -> u32 {
    LEGACY
        .iter()
        .filter(|(legacy, _)| modifiers & legacy != 0)
        .fold(modifiers & modifier::KEYBOARD, |all, (_, extended)| {
            all | extended
        })
}

/// Name of a key, as `KeyEvent.getKeyText` gives it in English.
pub fn key_text(key_code: u32) -> String {
    let name = match key_code {
        key::BACK_SPACE => "Backspace",
        key::TAB => "Tab",
        key::ENTER => "Enter",
        key::ESCAPE => "Escape",
        key::SPACE => "Space",
        key::PAGE_UP => "Page Up",
        key::PAGE_DOWN => "Page Down",
        key::END => "End",
        key::HOME => "Home",
        key::LEFT => "Left",
        key::UP => "Up",
        key::RIGHT => "Right",
        key::DOWN => "Down",
        key::COMMA => "Comma",
        key::MINUS => "Minus",
        key::PERIOD => "Period",
        key::SLASH => "Slash",
        key::SEMICOLON => "Semicolon",
        key::EQUALS => "Equals",
        key::OPEN_BRACKET => "Open Bracket",
        key::BACK_SLASH => "Back Slash",
        key::CLOSE_BRACKET => "Close Bracket",
        key::MULTIPLY => "NumPad *",
        key::ADD => "NumPad +",
        key::SUBTRACT => "NumPad -",
        key::DECIMAL => "NumPad .",
        key::DIVIDE => "NumPad /",
        key::DELETE => "Delete",
        key::INSERT => "Insert",
        key::BACK_QUOTE => "Back Quote",
        key::QUOTE => "Quote",
        code @ (48..=57 | 65..=90) => return char::from_u32(code).unwrap_or('?').to_string(),
        code @ key::NUMPAD0..=105 => return format!("NumPad-{}", code - key::NUMPAD0),
        code @ key::F1..=123 => return format!("F{}", code - key::F1 + 1),
        code => return format!("Unknown keyCode: 0x{code:x}"),
    };
    name.to_owned()
}

/// AWT key code of a browser key event, from its `key` and `code` properties; 0 when the key has
/// no AWT equivalent here.
///
/// NOTE: letters and digits follow the keyboard layout (`key`), like AWT; other keys are taken
/// from their position (`code`), as `key` then depends on the modifiers held (Alt on macOS).
pub fn key_code_from_browser(key: &str, code: &str) -> u32 {
    let mut chars = key.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    };
    if let Some(n) = code
        .strip_prefix("Numpad")
        .and_then(|n| n.parse::<u32>().ok())
    {
        // With Num Lock off the key is a navigation key (Home, ArrowUp…), named like its code.
        return if single.is_some_and(|c| c.is_ascii_digit()) {
            key::NUMPAD0 + n
        } else {
            key_code_from_code(key)
        };
    }
    if let Some(c) = single.filter(char::is_ascii_alphanumeric) {
        return c.to_ascii_uppercase() as u32;
    }
    key_code_from_code(code)
}

/// AWT key code of a browser `code` (physical key).
fn key_code_from_code(code: &str) -> u32 {
    if let Some(rest) = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
    {
        return rest.chars().next().map_or(0, |c| c as u32);
    }
    if let Some(n) = code.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        return if (1..=12).contains(&n) {
            key::F1 + n - 1
        } else {
            0
        };
    }
    match code {
        "Backspace" => key::BACK_SPACE,
        "Tab" => key::TAB,
        "Enter" | "NumpadEnter" => key::ENTER,
        "Escape" => key::ESCAPE,
        "Space" => key::SPACE,
        "PageUp" => key::PAGE_UP,
        "PageDown" => key::PAGE_DOWN,
        "End" => key::END,
        "Home" => key::HOME,
        "ArrowLeft" => key::LEFT,
        "ArrowUp" => key::UP,
        "ArrowRight" => key::RIGHT,
        "ArrowDown" => key::DOWN,
        "Comma" => key::COMMA,
        "Minus" => key::MINUS,
        "Period" => key::PERIOD,
        "Slash" => key::SLASH,
        "Semicolon" => key::SEMICOLON,
        "Equal" => key::EQUALS,
        "BracketLeft" => key::OPEN_BRACKET,
        "Backslash" => key::BACK_SLASH,
        "BracketRight" => key::CLOSE_BRACKET,
        "NumpadMultiply" => key::MULTIPLY,
        "NumpadAdd" => key::ADD,
        "NumpadSubtract" => key::SUBTRACT,
        "NumpadDecimal" => key::DECIMAL,
        "NumpadDivide" => key::DIVIDE,
        "Delete" => key::DELETE,
        "Insert" => key::INSERT,
        "Backquote" => key::BACK_QUOTE,
        "Quote" => key::QUOTE,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_modifiers_and_mouse_bits_are_normalized() {
        let ctrl_shift = modifier::CTRL | modifier::SHIFT;
        assert_eq!(KeyBinding::new('S' as u32, 2 | 1).modifiers, ctrl_shift);
        // BUTTON1_DOWN_MASK (1 << 10) is not a keyboard modifier.
        assert_eq!(
            KeyBinding::new('S' as u32, ctrl_shift | 1 << 10).modifiers,
            ctrl_shift
        );
    }

    #[test]
    fn a_binding_matches_exactly_its_modifiers() {
        let alt_l = KeyBinding::new('L' as u32, modifier::ALT);
        assert!(alt_l.matches('L' as u32, modifier::ALT));
        assert!(!alt_l.matches('L' as u32, modifier::ALT | modifier::CTRL));
        assert!(!alt_l.matches('L' as u32, 0));
        assert!(
            !KeyBinding::NONE.matches(0, 0),
            "unassigned matches nothing"
        );
    }

    #[test]
    fn texts_follow_awt() {
        let binding = KeyBinding::new('S' as u32, modifier::SHIFT | modifier::CTRL);
        assert_eq!(binding.text(), "Ctrl+Shift+S");
        assert_eq!(
            KeyBinding::new(key::ADD, modifier::CTRL).text(),
            "Ctrl+NumPad +"
        );
        assert_eq!(KeyBinding::new(key::PAGE_UP, 0).text(), "Page Up");
        assert_eq!(KeyBinding::new(key::F1 + 10, 0).text(), "F11");
        assert_eq!(KeyBinding::new(key::NUMPAD0 + 7, 0).text(), "NumPad-7");
        assert_eq!(KeyBinding::NONE.text(), "");
    }

    #[test]
    fn browser_keys_become_awt_codes() {
        assert_eq!(
            key_code_from_browser("a", "KeyQ"),
            'A' as u32,
            "layout of the user"
        );
        assert_eq!(
            key_code_from_browser("¬", "KeyL"),
            'L' as u32,
            "Alt on macOS"
        );
        assert_eq!(key_code_from_browser("ArrowUp", "ArrowUp"), key::UP);
        assert_eq!(key_code_from_browser("+", "NumpadAdd"), key::ADD);
        assert_eq!(key_code_from_browser("7", "Numpad7"), key::NUMPAD0 + 7);
        assert_eq!(
            key_code_from_browser("Home", "Numpad7"),
            key::HOME,
            "Num Lock off"
        );
        assert_eq!(key_code_from_browser("&", "Digit1"), '1' as u32, "AZERTY");
        assert_eq!(key_code_from_browser("F11", "F11"), key::F1 + 10);
        assert_eq!(key_code_from_browser(" ", "Space"), key::SPACE);
        assert_eq!(key_code_from_browser("Shift", "ShiftLeft"), 0);
    }
}
