//! # aa-linux
//!
//! The Linux wallpaper shell. There is no universal "wallpaper" API, so this
//! crate picks a backend at runtime:
//!
//! * **Wayland** (`WAYLAND_DISPLAY` set): a `wlr-layer-shell` surface on the
//!   `background` layer, one per output. Works on wlroots compositors (sway,
//!   Hyprland) and KDE. GNOME-Wayland is **out of scope** (needs a Shell
//!   extension) — see the cross-platform plan.
//! * **X11** (fallback): paint the root window pixmap and set `_XROOTPMAP_ID` /
//!   `ESETROOT_PMAP_ID`, the convention feh/conky use; multi-monitor via RandR.
//!
//! STATUS: skeleton. Real implementation is `#[cfg(target_os = "linux")]` and
//! built on CI (`ubuntu`); elsewhere this crate is an empty stub.

/// Which wallpaper backend to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
}

/// Pick a backend from the environment: Wayland if `WAYLAND_DISPLAY` is set,
/// else X11.
pub fn detect_backend() -> Backend {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        Backend::Wayland
    } else {
        Backend::X11
    }
}

#[derive(Debug)]
pub enum ShellError {
    Unsupported,
    #[cfg(target_os = "linux")]
    Backend(String),
}

/// Launch the wallpaper host on the detected backend and run the render loop.
pub fn run() -> Result<(), ShellError> {
    #[cfg(target_os = "linux")]
    {
        imp::run(detect_backend())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(ShellError::Unsupported)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Backend, ShellError};

    // TODO(aa-linux):
    //   wayland: wlr-layer-shell background surface per wl_output; shm/GPU buffer
    //            fed by aa_render::render(); StatusNotifier (ksni) tray on KDE.
    //   x11:     create a pixmap, blit aa_render output, set _XROOTPMAP_ID and
    //            ESETROOT_PMAP_ID on the root; RandR for per-monitor geometry.
    pub fn run(backend: Backend) -> Result<(), ShellError> {
        Err(ShellError::Backend(format!("{backend:?} backend not yet implemented")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_detection_is_total() {
        // Just exercise the path; result depends on the host env.
        let _ = detect_backend();
    }
}
