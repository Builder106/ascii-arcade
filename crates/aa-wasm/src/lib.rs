//! Browser bindings for `aa-core`. The marketing site runs the shipping scene
//! engine through this rather than a JavaScript port, so the two cannot drift.

use aa_core::{scenes, RgbColor, Scene, Theme};
use wasm_bindgen::prelude::*;

// Errors are `String`, not `JsValue`. wasm-bindgen accepts any error that
// converts into a JsValue, and constructing a JsValue outside wasm32 aborts the
// process, which would make the error path untestable under `cargo test`.
type SceneResult<T> = Result<T, String>;

/// One live scene plus its most recent frame, flattened for JavaScript.
#[wasm_bindgen]
pub struct Engine {
    scene: Box<dyn Scene + Send>,
    glyphs: String,
    colors: Vec<u32>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, cols: usize, rows: usize) -> SceneResult<Engine> {
        let mut scene = scenes::make(id).ok_or_else(|| format!("unknown scene: {id}"))?;
        scene.set_grid(cols, rows);
        Ok(Engine {
            scene,
            glyphs: String::new(),
            colors: Vec::new(),
        })
    }

    pub fn set_grid(&mut self, cols: usize, rows: usize) {
        self.scene.set_grid(cols, rows);
    }

    pub fn apply_base_color(&mut self, r: u8, g: u8, b: u8) {
        self.scene.apply_base_color(RgbColor::new(r, g, b));
    }

    /// Advance to `t` seconds and cache the frame. Call before the getters.
    pub fn render(&mut self, t: f64) {
        let frame = self.scene.frame(t);
        self.glyphs.clear();
        self.colors.clear();
        self.colors.reserve(frame.cells.len());
        for cell in &frame.cells {
            self.glyphs.push(cell.ch);
            // Packed explicitly rather than via RgbColor::to_argb so that a
            // genuinely black cell can never collide with the zero sentinel.
            self.colors.push(match cell.color {
                Some(c) => {
                    0xFF00_0000 | (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
                }
                None => 0,
            });
        }
    }

    /// Row-major glyphs, `cols * rows` characters, no newlines.
    pub fn glyphs(&self) -> String {
        self.glyphs.clone()
    }

    /// Row-major colour, one entry per cell. Zero means "use the theme colour".
    pub fn colors(&self) -> Vec<u32> {
        self.colors.clone()
    }
}

/// Built-in scene ids, in the order the site presents them.
#[wasm_bindgen]
pub fn scene_ids() -> Vec<String> {
    scenes::BUILTIN_IDS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// The four palettes, so the page never keeps a second copy that can drift.
/// Hand-rolled rather than pulling in serde, which would cost more than it saves.
#[wasm_bindgen]
pub fn themes_json() -> String {
    let entries: Vec<String> = Theme::ALL
        .iter()
        .map(|t| {
            format!(
                r#"{{"name":"{}","text":[{},{},{}],"background":[{},{},{}]}}"#,
                t.name,
                t.text.r,
                t.text.g,
                t.text.b,
                t.background.r,
                t.background.g,
                t.background.b
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_renders_a_full_grid() {
        let mut e = Engine::new("donut", 40, 20).expect("donut exists");
        e.render(1.0);
        assert_eq!(e.glyphs().chars().count(), 800);
        assert_eq!(e.colors().len(), 800);
    }

    #[test]
    fn unknown_scene_is_an_error() {
        assert!(Engine::new("nope", 10, 10).is_err());
    }

    #[test]
    fn uncoloured_cells_use_zero_sentinel() {
        let mut e = Engine::new("donut", 20, 10).expect("donut exists");
        e.render(0.5);
        assert!(e.colors().iter().any(|&c| c == 0), "donut is monochrome");
    }

    #[test]
    fn themes_json_lists_all_four() {
        let json = themes_json();
        for name in ["Hacker", "Amber", "Ice", "Ghost"] {
            assert!(json.contains(name), "missing {name}");
        }
    }
}
