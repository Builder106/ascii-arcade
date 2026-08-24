//! Colour themes, ported from the macOS host's `availableThemes`.
//!
//! The macOS shell remains the source of truth and passes its own resolved text
//! colour via [`crate::scene::Scene::apply_base_color`]; these constants let the
//! Windows/Linux shells stand alone with matching palettes. (`Hacker` / `Ice`
//! used the dynamic `NSColor.systemGreen` / `.cyan`, approximated here.)

use crate::color::RgbColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub text: RgbColor,
    pub background: RgbColor,
}

impl Theme {
    pub const HACKER: Theme = Theme {
        name: "Hacker",
        text: RgbColor::new(48, 209, 88), // NSColor.systemGreen
        background: RgbColor::BLACK,
    };
    pub const AMBER: Theme = Theme {
        name: "Amber",
        text: RgbColor::new(255, 166, 0),
        background: RgbColor::new(26, 8, 0),
    };
    pub const ICE: Theme = Theme {
        name: "Ice",
        text: RgbColor::new(0, 255, 255), // NSColor.cyan
        background: RgbColor::new(0, 13, 26),
    };
    pub const GHOST: Theme = Theme {
        name: "Ghost",
        text: RgbColor::new(28, 28, 30),
        background: RgbColor::new(245, 245, 245),
    };

    /// All themes in the macOS host's order (Hacker is the first-run default).
    pub const ALL: [Theme; 4] = [Theme::HACKER, Theme::AMBER, Theme::ICE, Theme::GHOST];

    pub fn by_name(name: &str) -> Option<Theme> {
        Theme::ALL
            .into_iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::HACKER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_hacker() {
        assert_eq!(Theme::default(), Theme::HACKER);
    }

    #[test]
    fn by_name_finds_all_themes_case_insensitively() {
        assert_eq!(Theme::by_name("hacker"), Some(Theme::HACKER));
        assert_eq!(Theme::by_name("HACKER"), Some(Theme::HACKER));
        assert_eq!(Theme::by_name("Hacker"), Some(Theme::HACKER));

        assert_eq!(Theme::by_name("amber"), Some(Theme::AMBER));
        assert_eq!(Theme::by_name("Amber"), Some(Theme::AMBER));

        assert_eq!(Theme::by_name("ice"), Some(Theme::ICE));
        assert_eq!(Theme::by_name("ICE"), Some(Theme::ICE));

        assert_eq!(Theme::by_name("ghost"), Some(Theme::GHOST));
        assert_eq!(Theme::by_name("GhOsT"), Some(Theme::GHOST));

        assert_eq!(Theme::by_name("nonexistent"), None);
        assert_eq!(Theme::by_name(""), None);
    }

    #[test]
    fn all_contains_expected_themes_and_count() {
        assert_eq!(Theme::ALL.len(), 4);
        assert_eq!(Theme::ALL[0], Theme::HACKER);
        assert_eq!(Theme::ALL[1], Theme::AMBER);
        assert_eq!(Theme::ALL[2], Theme::ICE);
        assert_eq!(Theme::ALL[3], Theme::GHOST);

        for theme in Theme::ALL {
            assert!(!theme.name.is_empty());
            assert_eq!(Theme::by_name(theme.name), Some(theme));
        }
    }
}
