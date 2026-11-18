//! Slot surface themes.

/// A complete slot surface theme.
pub trait SlotTheme: Send + Sync + 'static {
    /// Returns the stable theme id.
    fn id(&self) -> &'static str;
    /// Returns the theme rules.
    fn css(&self) -> &'static str;
}

/// A theme selected at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub id: &'static str,
    pub css: &'static str,
}

impl Theme {
    /// Creates a theme from an id and its rules.
    pub const fn new(id: &'static str, css: &'static str) -> Self {
        Self { id, css }
    }
}

impl SlotTheme for Theme {
    fn id(&self) -> &'static str {
        self.id
    }

    fn css(&self) -> &'static str {
        self.css
    }
}

impl Default for Theme {
    fn default() -> Self {
        DARK
    }
}

/// Dark slot theme.
pub const DARK: Theme = bevy_dioxus_slots_macro::theme!("../../assets/themes/dark.css");
/// Light slot theme.
pub const LIGHT: Theme = bevy_dioxus_slots_macro::theme!("../../assets/themes/light.css");

/// Returns whether css defines a rule for a selector.
#[doc(hidden)]
pub const fn css_has_rule(css: &str, selector: &str) -> bool {
    let css = css.as_bytes();
    let selector = selector.as_bytes();
    if selector.is_empty() || selector.len() > css.len() {
        return false;
    }
    let mut index = 0;
    let mut in_comment = false;
    let mut quote = 0u8;
    while index < css.len() {
        let byte = css[index];
        if in_comment {
            if byte == b'*' && index + 1 < css.len() && css[index + 1] == b'/' {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if quote != 0 {
            if byte == b'\\' && index + 1 < css.len() {
                index += 2;
                continue;
            }
            if byte == quote {
                quote = 0;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && index + 1 < css.len() && css[index + 1] == b'*' {
            in_comment = true;
            index += 2;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            quote = byte;
            index += 1;
            continue;
        }
        if starts_with_at(css, index, selector) {
            let mut before = index;
            let mut boundary = true;
            while before > 0 {
                let previous = css[before - 1];
                if is_space(previous) {
                    before -= 1;
                    continue;
                }
                boundary = matches!(previous, b'{' | b'}' | b';' | b',');
                break;
            }
            let mut after = index + selector.len();
            while after < css.len() && is_space(css[after]) {
                after += 1;
            }
            let followed = after < css.len() && (css[after] == b'{' || css[after] == b',');
            if boundary && followed {
                return true;
            }
        }
        index += 1;
    }
    false
}

const fn starts_with_at(bytes: &[u8], start: usize, needle: &[u8]) -> bool {
    if start + needle.len() > bytes.len() {
        return false;
    }
    let mut offset = 0;
    while offset < needle.len() {
        if bytes[start + offset] != needle[offset] {
            return false;
        }
        offset += 1;
    }
    true
}

const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_distinct() {
        assert_ne!(DARK, LIGHT);
        assert_eq!(DARK.id, "dark");
        assert_eq!(LIGHT.id, "light");
        assert!(DARK.css.contains(".slot-title"));
        assert!(LIGHT.css.contains(".slot-title"));
    }

    #[test]
    fn default_theme_is_dark() {
        assert_eq!(Theme::default(), DARK);
    }

    #[test]
    fn rule_detection_handles_groups_comments_and_strings() {
        assert!(css_has_rule(".a, .b {}", ".b"));
        assert!(css_has_rule(".a\n{\n}", ".a"));
        assert!(!css_has_rule("/* .a {} */", ".a"));
        assert!(!css_has_rule(".x { content: \".a {}\"; }", ".a"));
        assert!(!css_has_rule(".alpha {}", ".a"));
    }
}
