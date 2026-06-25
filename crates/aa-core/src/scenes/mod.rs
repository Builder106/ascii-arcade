//! The built-in scene catalogue.
//!
//! Each scene is a port of its `*Scene.swift` / `*FrameGenerator.swift`
//! counterpart. `donut` is implemented as the reference; the rest are ported
//! against the same [`crate::scene::Scene`] contract.

pub mod donut;
pub use donut::DonutScene;

// Ported incrementally; uncomment each module as it lands.
// pub mod helix;
// pub mod fire;
// pub mod matrix;
// pub mod life;
// pub mod pipes;
// pub mod clock;
// pub mod stepped;

use crate::scene::Scene;

/// Stable identifiers for the built-in scenes (used by shells to persist the
/// user's choice and by the headless renderer's CLI).
pub const BUILTIN_IDS: &[&str] = &["donut"];

/// Construct a built-in scene by id. Returns `None` for unknown ids.
pub fn make(id: &str) -> Option<Box<dyn Scene>> {
    match id {
        "donut" => Some(Box::new(DonutScene::new())),
        _ => None,
    }
}
