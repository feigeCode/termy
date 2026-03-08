#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeColors {
    pub ansi: [Rgb8; 16],
    pub foreground: Rgb8,
    pub background: Rgb8,
    pub cursor: Rgb8,
}

pub const BUILTIN_THEME_IDS: &[&str] = &[];

pub const ANSI_COLOR_NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright_black",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "bright_white",
];

pub fn normalize_theme_id(theme_id: &str) -> String {
    let mut normalized = String::new();
    let mut last_dash = false;

    for ch in theme_id.trim().chars() {
        let ch = ch.to_ascii_lowercase();
        match ch {
            'a'..='z' | '0'..='9' => {
                normalized.push(ch);
                last_dash = false;
            }
            '-' | '_' | ' ' => {
                if !normalized.is_empty() && !last_dash {
                    normalized.push('-');
                    last_dash = true;
                }
            }
            _ => {}
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    normalized
}

pub fn canonical_builtin_theme_id(theme_id: &str) -> Option<&'static str> {
    let _ = theme_id;
    None
}

pub fn format_hex(color: Rgb8) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::{Rgb8, canonical_builtin_theme_id, format_hex, normalize_theme_id};

    #[test]
    fn formats_hex_in_lowercase() {
        assert_eq!(format_hex(Rgb8::new(0xAB, 0xCD, 0xEF)), "#abcdef");
    }

    #[test]
    fn normalize_theme_id_is_stable() {
        assert_eq!(normalize_theme_id("  Tokyo_Night  "), "tokyo-night");
        assert_eq!(normalize_theme_id("gruvbox---dark"), "gruvbox-dark");
    }

    #[test]
    fn builtin_aliases_are_disabled() {
        assert_eq!(canonical_builtin_theme_id("gruvbox"), None);
        assert_eq!(canonical_builtin_theme_id("tokyonight"), None);
        assert_eq!(canonical_builtin_theme_id("default"), None);
    }
}
