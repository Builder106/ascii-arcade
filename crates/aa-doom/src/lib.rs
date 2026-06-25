//! # aa-doom
//!
//! DOOM as a [`Scene`](aa_core::Scene). Spawns the vendored `doom_ascii` binary
//! over a portable PTY and reconstructs its ANSI output into [`Frame`]s.
//!
//! This replaces the Darwin-only `PTYBridge` (`forkpty`) with `portable-pty`,
//! which speaks ConPTY on Windows and forkpty on macOS/Linux — the single change
//! that makes DOOM-on-Windows possible.
//!
//! STATUS: skeleton. Implemented by the aa-doom work-stream. Two pieces to port:
//!   1. `DoomLauncher.swift`  -> resolve the binary + IWAD, build argv/env.
//!   2. `DoomScreenBuffer.swift` -> minimal ANSI parser (cursor moves + clears,
//!      strips SGR) reconstructing the 320x200-derived character grid.

use aa_core::{Frame, Scene};

/// DOOM's internal framebuffer (`SCREENWIDTH x SCREENHEIGHT`).
pub const SCREEN_WIDTH: usize = 320;
pub const SCREEN_HEIGHT: usize = 200;

/// Character-grid dimensions `doom_ascii -scaling N` emits: each pixel becomes
/// two block chars wide, one row per scanline. Ported from `DoomLauncher`.
pub fn grid_size(scaling: usize) -> (usize, usize) {
    let n = scaling.max(1);
    ((SCREEN_WIDTH / n) * 2, SCREEN_HEIGHT / n)
}

/// A DOOM scene driven by a PTY-backed `doom_ascii` process.
pub struct DoomScene {
    scaling: usize,
    grid: (usize, usize),
}

impl DoomScene {
    pub fn new(scaling: usize) -> Self {
        DoomScene {
            scaling,
            grid: grid_size(scaling),
        }
    }

    /// The `-scaling N` factor `doom_ascii` is launched with.
    pub fn scaling(&self) -> usize {
        self.scaling
    }
}

impl Default for DoomScene {
    fn default() -> Self {
        DoomScene::new(1)
    }
}

impl Scene for DoomScene {
    fn display_name(&self) -> &str {
        "DOOM"
    }
    fn is_interactive(&self) -> bool {
        true
    }
    fn fixed_grid(&self) -> Option<(usize, usize)> {
        Some(self.grid)
    }
    fn set_grid(&mut self, _width: usize, _height: usize) {
        // DOOM has a fixed framebuffer; the shell scales the bitmap to fit.
    }
    fn frame(&mut self, _t: f64) -> Frame {
        // TODO(aa-doom): return the latest PTY-reconstructed frame.
        Frame::blank(self.grid.0, self.grid.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_size_matches_doomlauncher() {
        assert_eq!(grid_size(1), (640, 200));
        assert_eq!(grid_size(2), (320, 100));
    }
}
