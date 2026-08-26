//! The `Scene` trait — the single extension point of the engine.
//!
//! Ported from `AsciiScene` (Swift). The pull/push distinction (math scenes
//! compute on demand; DOOM streams from a PTY) is hidden behind one method:
//! [`Scene::frame`] takes `&mut self` so stateful scenes (Matrix, Life,
//! pipes, DOOM) can advance their simulation, and returns a fully-realised
//! [`Frame`] carrying optional per-cell colour.

use crate::color::RgbColor;
use crate::frame::Frame;

/// One discrete choice within a [`SceneSetting`] (e.g. "Fast" → 2.0).
#[derive(Clone, Debug, PartialEq)]
pub struct SceneOption {
    pub label: String,
    pub value: f64,
}

impl SceneOption {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        SceneOption {
            label: label.into(),
            value,
        }
    }
}

/// A scene-tunable knob, surfaced by each shell as a menu of discrete presets
/// (the macOS shell renders these as checkmarked `NSMenu` items).
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSetting {
    pub id: String,
    pub label: String,
    pub options: Vec<SceneOption>,
    pub default_index: usize,
}

impl SceneSetting {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<SceneOption>,
        default_index: usize,
    ) -> Self {
        let len = options.len();
        SceneSetting {
            id: id.into(),
            label: label.into(),
            options,
            default_index: default_index.min(len.saturating_sub(1)),
        }
    }
}

/// A selectable ASCII experience the wallpaper shell can render.
///
/// Defaults mirror the Swift protocol extension so a simple math scene only has
/// to implement `display_name`, `set_grid`, and `frame`.
pub trait Scene {
    /// Human-readable name shown in the scene picker.
    fn display_name(&self) -> &str;

    /// Whether the scene consumes keyboard input. DOOM does; math scenes don't.
    fn is_interactive(&self) -> bool {
        false
    }

    /// Resize the character grid the scene renders into.
    fn set_grid(&mut self, width: usize, height: usize);

    /// A scene with a *fixed* pixel resolution (DOOM's framebuffer) returns its
    /// grid here; the shell then paints it as a scaled colour bitmap filling the
    /// screen rather than as font glyphs bound to the text grid. `None` (default)
    /// ⇒ a normal text scene driven by `set_grid`.
    fn fixed_grid(&self) -> Option<(usize, usize)> {
        None
    }

    /// The current frame at animation time `t` (seconds).
    fn frame(&mut self, t: f64) -> Frame;

    /// Tell the scene the theme's text colour so colour scenes can key their
    /// palette off it (e.g. Matrix rain turns amber under the Amber theme).
    fn apply_base_color(&mut self, _color: RgbColor) {}

    /// User-tunable knobs for this scene.
    fn settings(&self) -> Vec<SceneSetting> {
        Vec::new()
    }

    /// Apply a value chosen for one of [`Scene::settings`].
    fn apply_setting(&mut self, _id: &str, _value: f64) {}

    /// Forward raw key bytes to the scene (DOOM).
    fn send_key(&mut self, _bytes: &[u8]) {}

    /// Begin any backing work (spawning the DOOM process, etc.).
    fn start(&mut self) {}

    /// Stop and tear down any backing work.
    fn stop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalScene {
        width: usize,
        height: usize,
    }

    impl Scene for MinimalScene {
        fn display_name(&self) -> &str {
            "Minimal"
        }

        fn set_grid(&mut self, width: usize, height: usize) {
            self.width = width;
            self.height = height;
        }

        fn frame(&mut self, _t: f64) -> Frame {
            Frame::blank(self.width, self.height)
        }
    }

    #[test]
    fn test_scene_option_new() {
        let opt = SceneOption::new("Fast", 2.0);
        assert_eq!(opt.label, "Fast");
        assert_eq!(opt.value, 2.0);
    }

    #[test]
    fn test_scene_setting_new_clamping() {
        // Normal valid index
        let opt1 = SceneOption::new("Slow", 1.0);
        let opt2 = SceneOption::new("Fast", 2.0);
        let setting = SceneSetting::new("speed", "Speed", vec![opt1.clone(), opt2.clone()], 1);
        assert_eq!(setting.id, "speed");
        assert_eq!(setting.label, "Speed");
        assert_eq!(setting.default_index, 1);
        assert_eq!(setting.options.len(), 2);

        // Clamping out-of-bounds default_index
        let setting_clamped = SceneSetting::new("speed", "Speed", vec![opt1, opt2], 10);
        assert_eq!(setting_clamped.default_index, 1);

        // Empty options list
        let empty_setting = SceneSetting::new("empty", "Empty", Vec::new(), 5);
        assert_eq!(empty_setting.default_index, 0);
        assert!(empty_setting.options.is_empty());
    }

    #[test]
    fn test_scene_trait_default_implementations() {
        let mut scene = MinimalScene {
            width: 10,
            height: 5,
        };
        assert_eq!(scene.display_name(), "Minimal");
        assert!(!scene.is_interactive());
        assert_eq!(scene.fixed_grid(), None);
        assert!(scene.settings().is_empty());

        // Default no-op methods should not panic
        scene.apply_base_color(RgbColor::new(255, 0, 0));
        scene.apply_setting("any_key", 1.0);
        scene.send_key(b"abc");
        scene.start();
        scene.stop();

        scene.set_grid(20, 10);
        let frame = scene.frame(0.0);
        assert_eq!(frame.width, 20);
        assert_eq!(frame.height, 10);
    }
}
