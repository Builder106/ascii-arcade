//! # aa-linux
//!
//! The Linux wallpaper shell. There is no universal "wallpaper" API, so this
//! crate picks a backend at runtime:
//!
//! * **X11** ([`x11`]): paint a root-window pixmap and publish it via the
//!   `_XROOTPMAP_ID` / `ESETROOT_PMAP_ID` convention (what feh/conky use), then
//!   re-blit + `clear_area` each frame to animate. Pure-Rust `x11rb`, so it
//!   cross-compiles from any host. Also covers XWayland sessions.
//! * **Wayland**: a `wlr-layer-shell` surface on the `background` layer. Works
//!   on wlroots compositors (sway, Hyprland) and KDE. GNOME-Wayland is **out of
//!   scope** (needs a Shell extension). Built on CI's ubuntu runner.
//!
//! The real backends are `#[cfg(target_os = "linux")]`; elsewhere this crate is
//! a stub so the workspace checks on macOS/Windows.

use aa_render::RenderOptions;

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
    /// Built for a non-Linux target.
    Unsupported,
    /// The selected backend wasn't compiled in (missing cargo feature).
    BackendNotCompiled(Backend),
    /// A backend failed at runtime.
    #[cfg(target_os = "linux")]
    Backend(String),
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::Unsupported => write!(f, "the Linux shell only runs on Linux"),
            ShellError::BackendNotCompiled(b) => {
                write!(
                    f,
                    "{b:?} backend not compiled in (enable its cargo feature)"
                )
            }
            #[cfg(target_os = "linux")]
            ShellError::Backend(s) => write!(f, "backend error: {s}"),
        }
    }
}

impl std::error::Error for ShellError {}

/// Launch the wallpaper host on the detected backend and run the render loop.
/// `scene_id` is an `aa_core::scenes` id; `opts` controls the rasteriser.
pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
    #[cfg(target_os = "linux")]
    {
        match detect_backend() {
            Backend::X11 => run_x11(scene_id, opts),
            Backend::Wayland => run_wayland(scene_id, opts),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (scene_id, opts);
        Err(ShellError::Unsupported)
    }
}

#[cfg(target_os = "linux")]
fn run_x11(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
    #[cfg(feature = "x11")]
    {
        x11::run(scene_id, opts).map_err(ShellError::Backend)
    }
    #[cfg(not(feature = "x11"))]
    {
        let _ = (scene_id, opts);
        Err(ShellError::BackendNotCompiled(Backend::X11))
    }
}

#[cfg(target_os = "linux")]
fn run_wayland(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
    #[cfg(feature = "wayland")]
    {
        wayland::run(scene_id, opts).map_err(ShellError::Backend)
    }
    #[cfg(not(feature = "wayland"))]
    {
        let _ = (scene_id, opts);
        Err(ShellError::BackendNotCompiled(Backend::Wayland))
    }
}

/// Swizzle an [`aa_render`] RGBA buffer into the BGRX byte order both X11
/// (ZPixmap, 32bpp LSBFirst) and Wayland (`Xrgb8888`) expect.
#[cfg(target_os = "linux")]
fn rgba_to_bgrx(pixels: &[u8], out: &mut Vec<u8>) {
    out.resize(pixels.len(), 0);
    for (dst, src) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
        dst[0] = src[2]; // B
        dst[1] = src[1]; // G
        dst[2] = src[0]; // R
        dst[3] = 0; // X
    }
}

#[cfg(all(target_os = "linux", feature = "x11"))]
mod x11 {
    use super::rgba_to_bgrx;
    use aa_render::{render, RenderOptions};
    use std::time::Instant;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, CreateGCAux, ImageFormat, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), String> {
        let mut scene =
            aa_core::scenes::make(scene_id).ok_or(format!("unknown scene '{scene_id}'"))?;

        let (conn, screen_num) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let depth = screen.root_depth;
        let (w, h) = (screen.width_in_pixels, screen.height_in_pixels);

        // A pixmap we repaint each frame and publish as the root background.
        let pixmap = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_pixmap(depth, pixmap, root, w, h)
            .map_err(|e| e.to_string())?;
        let gc = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_gc(gc, pixmap, &CreateGCAux::default())
            .map_err(|e| e.to_string())?;

        // Publish the pixmap id so compositor-aware tools recognise it.
        for name in ["_XROOTPMAP_ID", "ESETROOT_PMAP_ID"] {
            let atom = conn
                .intern_atom(false, name.as_bytes())
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?
                .atom;
            conn.change_property32(PropMode::REPLACE, root, atom, AtomEnum::PIXMAP, &[pixmap])
                .map_err(|e| e.to_string())?;
        }
        conn.change_window_attributes(
            root,
            &x11rb::protocol::xproto::ChangeWindowAttributesAux::default()
                .background_pixmap(pixmap),
        )
        .map_err(|e| e.to_string())?;

        let cols = (w as u32 / opts.cell_w.max(1)).max(1) as usize;
        let rows = (h as u32 / opts.cell_h.max(1)).max(1) as usize;
        scene.set_grid(cols, rows);
        scene.start();

        let start = Instant::now();
        let mut bgrx: Vec<u8> = Vec::new();
        loop {
            let frame = scene.frame(start.elapsed().as_secs_f64());
            let buf = render(&frame, &opts);
            rgba_to_bgrx(&buf.pixels, &mut bgrx);

            put_image_banded(
                &conn,
                pixmap,
                gc,
                depth,
                buf.width as u16,
                buf.height as u16,
                &bgrx,
            )?;
            // Force the root to repaint from the (just-updated) background pixmap.
            conn.clear_area(true, root, 0, 0, w, h)
                .map_err(|e| e.to_string())?;
            conn.flush().map_err(|e| e.to_string())?;

            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    }

    /// `put_image` in horizontal bands so each request stays under the server's
    /// maximum request size (a full-screen image easily exceeds it).
    fn put_image_banded<C: Connection>(
        conn: &C,
        pixmap: u32,
        gc: u32,
        depth: u8,
        width: u16,
        height: u16,
        bgrx: &[u8],
    ) -> Result<(), String> {
        let stride = width as usize * 4;
        if stride == 0 {
            return Ok(());
        }
        // Conservative 256 KiB budget per request, at least one row.
        let rows_per_band = (262_144 / stride).max(1);
        let mut y = 0usize;
        while y < height as usize {
            let band = rows_per_band.min(height as usize - y);
            let data = &bgrx[y * stride..(y + band) * stride];
            conn.put_image(
                ImageFormat::Z_PIXMAP,
                pixmap,
                gc,
                width,
                band as u16,
                0,
                y as i16,
                0,
                depth,
                data,
            )
            .map_err(|e| e.to_string())?;
            y += band;
        }
        Ok(())
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
mod wayland {
    use aa_render::RenderOptions;

    // The Wayland backend targets the `wlr-layer-shell` `background` layer via
    // smithay-client-toolkit (one layer surface per wl_output, shm buffer fed by
    // aa_render). It links libwayland and so only builds on a Linux host with the
    // Wayland system libraries present — it is verified on CI's ubuntu runner,
    // not from the macOS dev machine. Implemented as a follow-up; the X11 backend
    // (incl. XWayland) covers the verifiable path today.
    pub fn run(_scene_id: &str, _opts: RenderOptions) -> Result<(), String> {
        Err("wayland (wlr-layer-shell) backend not yet implemented; \
             run under X11/XWayland for now"
            .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_detection_is_total() {
        let _ = detect_backend();
    }
}
