//! # aa-core
//!
//! The platform-neutral ASCII Arcade engine, ported from the Swift
//! `AsciiArcadeCore` target. It has **no platform dependencies**: it produces
//! [`Frame`]s of coloured character cells that the native shells (macOS/AppKit,
//! Windows/WorkerW, Linux/X11+layer-shell) rasterise and blit.
//!
//! The single extension point is the [`Scene`] trait. A scene takes an animation
//! time and returns a [`Frame`]; the pull/push distinction between the math
//! scenes and PTY-backed DOOM is hidden behind it.

pub mod color;
pub mod frame;
pub mod rng;
pub mod scene;
pub mod scenes;
pub mod theme;

pub use color::RgbColor;
pub use frame::{Cell, Frame};
pub use rng::SeededRng;
pub use scene::{Scene, SceneOption, SceneSetting};
pub use theme::Theme;
