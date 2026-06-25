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
        // Colour scenes (Matrix, fire, …) key their palette off the theme colour.
        scene.apply_base_color(opts.theme.text);

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
    //! `wlr-layer-shell` `background`-layer backend via smithay-client-toolkit.
    //! A single layer surface fills the default output; frames are driven by
    //! `wl_surface.frame` callbacks and rendered into an shm buffer. (Multi-output
    //! is a documented follow-up.)

    use super::rgba_to_bgrx;
    use aa_render::{render, RenderOptions};
    use std::time::Instant;

    use smithay_client_toolkit::{
        compositor::{CompositorHandler, CompositorState},
        delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
        output::{OutputHandler, OutputState},
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
        shell::{
            wlr_layer::{
                Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
                LayerSurfaceConfigure,
            },
            WaylandSurface,
        },
        shm::{slot::SlotPool, Shm, ShmHandler},
    };
    use wayland_client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_shm, wl_surface},
        Connection, QueueHandle,
    };

    pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), String> {
        let mut scene =
            aa_core::scenes::make(scene_id).ok_or(format!("unknown scene '{scene_id}'"))?;
        // Colour scenes (Matrix, fire, …) key their palette off the theme colour.
        scene.apply_base_color(opts.theme.text);

        let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
        let (globals, mut event_queue) = registry_queue_init(&conn).map_err(|e| e.to_string())?;
        let qh = event_queue.handle();

        let compositor =
            CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?;
        let layer_shell =
            LayerShell::bind(&globals, &qh).map_err(|e| format!("wlr_layer_shell: {e}"))?;
        let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?;

        // A background layer surface anchored to all edges fills the output.
        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Background,
            Some("ascii-arcade"),
            None,
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(0, 0);
        layer.commit();

        let pool = SlotPool::new(256 * 256 * 4, &shm).map_err(|e| e.to_string())?;

        scene.start();
        let mut state = State {
            registry_state: RegistryState::new(&globals),
            output_state: OutputState::new(&globals, &qh),
            shm,
            pool,
            layer,
            width: 0,
            height: 0,
            closed: false,
            scene,
            opts,
            start: Instant::now(),
            scratch: Vec::new(),
        };

        while !state.closed {
            event_queue
                .blocking_dispatch(&mut state)
                .map_err(|e| e.to_string())?;
        }
        state.scene.stop();
        Ok(())
    }

    struct State {
        registry_state: RegistryState,
        output_state: OutputState,
        shm: Shm,
        pool: SlotPool,
        layer: LayerSurface,
        width: u32,
        height: u32,
        closed: bool,
        scene: Box<dyn aa_core::Scene>,
        opts: RenderOptions,
        start: Instant,
        scratch: Vec<u8>,
    }

    impl State {
        fn draw(&mut self, qh: &QueueHandle<Self>) {
            let w = self.width.max(1);
            let h = self.height.max(1);
            let stride = w as i32 * 4;

            let (buffer, canvas) =
                match self
                    .pool
                    .create_buffer(w as i32, h as i32, stride, wl_shm::Format::Xrgb8888)
                {
                    Ok(b) => b,
                    Err(_) => return,
                };

            let frame = self.scene.frame(self.start.elapsed().as_secs_f64());
            let buf = render(&frame, &self.opts);
            rgba_to_bgrx(&buf.pixels, &mut self.scratch);

            // Fill the theme background, then copy the rendered region (which can
            // be a few pixels smaller than the surface due to integer cell sizing).
            let bg = self.opts.theme.background;
            for px in canvas.chunks_exact_mut(4) {
                px[0] = bg.b;
                px[1] = bg.g;
                px[2] = bg.r;
                px[3] = 255;
            }
            let row = w as usize * 4;
            let bw = buf.width as usize;
            let cw = bw.min(w as usize) * 4;
            let ch = (buf.height as usize).min(h as usize);
            for y in 0..ch {
                canvas[y * row..y * row + cw]
                    .copy_from_slice(&self.scratch[y * bw * 4..y * bw * 4 + cw]);
            }

            let surface = self.layer.wl_surface();
            // Ask for the next frame before committing so animation continues.
            surface.frame(qh, surface.clone());
            if buffer.attach_to(surface).is_err() {
                return;
            }
            surface.damage_buffer(0, 0, w as i32, h as i32);
            surface.commit();
        }
    }

    impl CompositorHandler for State {
        fn scale_factor_changed(
            &mut self,
            _: &Connection,
            _: &QueueHandle<Self>,
            _: &wl_surface::WlSurface,
            _: i32,
        ) {
        }
        fn transform_changed(
            &mut self,
            _: &Connection,
            _: &QueueHandle<Self>,
            _: &wl_surface::WlSurface,
            _: wl_output::Transform,
        ) {
        }
        fn frame(
            &mut self,
            _: &Connection,
            qh: &QueueHandle<Self>,
            _: &wl_surface::WlSurface,
            _: u32,
        ) {
            self.draw(qh);
        }
        fn surface_enter(
            &mut self,
            _: &Connection,
            _: &QueueHandle<Self>,
            _: &wl_surface::WlSurface,
            _: &wl_output::WlOutput,
        ) {
        }
        fn surface_leave(
            &mut self,
            _: &Connection,
            _: &QueueHandle<Self>,
            _: &wl_surface::WlSurface,
            _: &wl_output::WlOutput,
        ) {
        }
    }

    impl LayerShellHandler for State {
        fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
            self.closed = true;
        }
        fn configure(
            &mut self,
            _: &Connection,
            qh: &QueueHandle<Self>,
            _: &LayerSurface,
            configure: LayerSurfaceConfigure,
            _serial: u32,
        ) {
            let (mut w, mut h) = configure.new_size;
            if w == 0 {
                w = 1920;
            }
            if h == 0 {
                h = 1080;
            }
            self.width = w;
            self.height = h;
            let cols = (w / self.opts.cell_w.max(1)).max(1) as usize;
            let rows = (h / self.opts.cell_h.max(1)).max(1) as usize;
            self.scene.set_grid(cols, rows);
            self.draw(qh);
        }
    }

    impl OutputHandler for State {
        fn output_state(&mut self) -> &mut OutputState {
            &mut self.output_state
        }
        fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
        fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        }
        fn output_destroyed(
            &mut self,
            _: &Connection,
            _: &QueueHandle<Self>,
            _: wl_output::WlOutput,
        ) {
        }
    }

    impl ShmHandler for State {
        fn shm_state(&mut self) -> &mut Shm {
            &mut self.shm
        }
    }

    impl ProvidesRegistryState for State {
        fn registry(&mut self) -> &mut RegistryState {
            &mut self.registry_state
        }
        registry_handlers![OutputState];
    }

    delegate_compositor!(State);
    delegate_output!(State);
    delegate_shm!(State);
    delegate_layer!(State);
    delegate_registry!(State);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_detection_is_total() {
        let _ = detect_backend();
    }
}
