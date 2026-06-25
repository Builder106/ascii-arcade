//! # aa-doom
//!
//! DOOM as a [`Scene`](aa_core::Scene). Spawns the vendored `doom_ascii` binary
//! over a portable PTY and reconstructs its ANSI output into coloured [`Frame`]s.
//!
//! This replaces the Darwin-only `PTYBridge` (`forkpty`) with `portable-pty`,
//! which speaks ConPTY on Windows and forkpty on macOS/Linux — the single change
//! that makes DOOM-on-Windows possible.
//!
//! Pieces:
//!   * [`launcher`] — resolve the binary + IWAD, build argv/env (port of
//!     `DoomLauncher.swift`).
//!   * [`screen`] — ANSI parser reconstructing the truecolor grid (port of
//!     `DoomScreenBuffer.swift`).
//!   * [`DoomScene`] — owns the PTY child + a reader thread that feeds the
//!     screen buffer; [`Scene::frame`] snapshots the latest grid.

pub mod launcher;
pub mod screen;

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use aa_core::{Frame, Scene};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

use screen::ScreenBuffer;

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
///
/// The screen buffer is shared with the reader thread behind a mutex; `frame`
/// just snapshots it, so rendering never blocks on the PTY.
pub struct DoomScene {
    scaling: usize,
    grid: (usize, usize),
    screen: Arc<Mutex<ScreenBuffer>>,
    running: bool,
    // Held to keep the PTY alive / tear it down on stop().
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    reader: Option<JoinHandle<()>>,
}

impl DoomScene {
    pub fn new(scaling: usize) -> Self {
        let grid = grid_size(scaling);
        DoomScene {
            scaling,
            grid,
            screen: Arc::new(Mutex::new(ScreenBuffer::new(grid.0, grid.1))),
            running: false,
            master: None,
            writer: None,
            child: None,
            reader: None,
        }
    }

    /// The `-scaling N` factor `doom_ascii` is launched with.
    pub fn scaling(&self) -> usize {
        self.scaling
    }

    fn set_message(&self, msg: &str) {
        if let Ok(mut s) = self.screen.lock() {
            s.show_message(msg);
        }
    }

    /// Spawn `doom_ascii` on a PTY sized to the character grid and start the
    /// reader thread. Returns without spawning (leaving an on-screen message) if
    /// the binary can't be resolved.
    fn launch(&mut self) -> std::io::Result<()> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let env: std::collections::HashMap<String, String> = std::env::vars().collect();

        let Some(config) = launcher::resolve(&cwd, &env, self.scaling) else {
            self.set_message("doom_ascii not found (run scripts/setup.sh)");
            return Ok(());
        };

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: self.grid.1 as u16,
                cols: self.grid.0 as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(to_io)?;

        let mut cmd = CommandBuilder::new(&config.executable);
        cmd.args(&config.args);
        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        cmd.cwd(&cwd);

        let child = pair.slave.spawn_command(cmd).map_err(to_io)?;
        // Drop the slave so the child holds the only slave handle; the master
        // then sees EOF when the child exits.
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(to_io)?;
        let writer = pair.master.take_writer().map_err(to_io)?;

        let screen = Arc::clone(&self.screen);
        let handle = std::thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut s) = screen.lock() {
                            s.feed(&chunk[..n]);
                        }
                    }
                }
            }
        });

        self.master = Some(pair.master);
        self.writer = Some(writer);
        self.child = Some(child);
        self.reader = Some(handle);
        self.running = true;
        Ok(())
    }
}

fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
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
        self.screen
            .lock()
            .map(|s| s.snapshot())
            .unwrap_or_else(|_| Frame::blank(self.grid.0, self.grid.1))
    }
    fn send_key(&mut self, bytes: &[u8]) {
        if let Some(w) = self.writer.as_mut() {
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }
    fn start(&mut self) {
        if self.running {
            return;
        }
        if let Err(e) = self.launch() {
            self.set_message(&format!("doom launch failed: {e}"));
        }
    }
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.writer = None;
        // Dropping the master closes the PTY, so the reader thread sees EOF.
        self.master = None;
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        self.running = false;
    }
}

impl Drop for DoomScene {
    fn drop(&mut self) {
        self.stop();
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

    #[test]
    fn fixed_grid_reports_scaled_dims() {
        let s = DoomScene::new(2);
        assert_eq!(s.fixed_grid(), Some((320, 100)));
    }

    #[test]
    fn frame_before_start_is_blank_grid() {
        let mut s = DoomScene::new(1);
        let f = s.frame(0.0);
        assert_eq!((f.width, f.height), (640, 200));
    }

    // End-to-end spawn is exercised by an ignored test (needs the binary +
    // mutates global cwd, so it's not part of the default parallel run):
    //   cargo test -p aa-doom -- --ignored spawns_doom_end_to_end
    #[test]
    #[ignore]
    fn spawns_doom_end_to_end() {
        let mut s = DoomScene::new(2);
        s.start();
        std::thread::sleep(std::time::Duration::from_millis(800));
        let non_blank = s.frame(0.0).cells.iter().filter(|c| c.ch != ' ').count();
        s.stop();
        assert!(non_blank > 0, "expected DOOM to render something");
    }
}
