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
        Theme::ALL.into_iter().find(|t| t.name.eq_ignore_ascii_case(name))
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::HACKER
    }
}
