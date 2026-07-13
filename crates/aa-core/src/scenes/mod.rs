//! The built-in scene catalogue.
//!
//! Each scene is a port of its `*Scene.swift` / `*FrameGenerator.swift`
//! counterpart, implemented against the shared [`crate::scene::Scene`] contract.
//! `stepped` is a shared fixed-timestep helper (ported from `SteppedScene.swift`)
//! that the stateful scenes use to map wall-clock `t` onto discrete sim steps.

pub mod stepped;

pub mod donut;
pub mod helix;
pub mod life;
pub mod matrix;
pub mod pipes;

pub use donut::DonutScene;
pub use helix::HelixScene;
pub use life::LifeScene;
pub use matrix::MatrixScene;
pub use pipes::PipesScene;

use crate::scene::Scene;

/// Stable identifiers for the built-in scenes (used by shells to persist the
/// user's choice and by the headless renderer's CLI). Order mirrors the macOS
/// host's scene menu.
pub const BUILTIN_IDS: &[&str] = &["donut", "helix", "matrix", "pipes", "life"];

/// Construct a built-in scene by id. Returns `None` for unknown ids.
///
/// The return type includes `Send` so scenes can be moved into async tasks
/// (e.g. the `aa-web` axum shell) without wrapping in a mutex.
pub fn make(id: &str) -> Option<Box<dyn Scene + Send>> {
    match id {
        "donut" => Some(Box::new(DonutScene::new())),
        "helix" => Some(Box::new(HelixScene::new())),
        "matrix" => Some(Box::new(MatrixScene::new())),
        "pipes" => Some(Box::new(PipesScene::new())),
        "life" => Some(Box::new(LifeScene::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_id_constructs() {
        for id in BUILTIN_IDS {
            assert!(make(id).is_some(), "make({id}) returned None");
        }
        assert!(make("nope").is_none());
    }

    #[test]
    fn builtins_render_at_a_given_grid() {
        for id in BUILTIN_IDS {
            let mut scene = make(id).unwrap();
            scene.set_grid(40, 20);
            let f = scene.frame(1.0);
            // DOOM-style fixed-grid scenes aren't here; all builtins honour set_grid.
            assert_eq!((f.width, f.height), (40, 20), "{id} ignored set_grid");
        }
    }
}
