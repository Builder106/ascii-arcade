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
//!   on wlroots compositors (sway, Hyprland) and KDE. GNOME uses the bundled
//!   Shell extension and the [`gnome`] frame transport instead of layer-shell.
//!
//! The real backends are `#[cfg(target_os = "linux")]`; elsewhere this crate is
//! a stub so the workspace checks on macOS/Windows.

use aa_render::RenderOptions;

pub mod gnome;

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
    run_with_doom(scene_id, opts, false)
}

/// Launch the Linux wallpaper host, exposing DOOM only when the user opted in.
pub fn run_with_doom(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
) -> Result<(), ShellError> {
    run_with_doom_settings(scene_id, opts, doom_enabled, &[])
}

pub fn run_with_doom_settings(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
) -> Result<(), ShellError> {
    run_with_controls(scene_id, opts, doom_enabled, settings, false, false)
}

pub fn run_with_controls(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
    idle_cycle: bool,
    capture_keys: bool,
) -> Result<(), ShellError> {
    #[cfg(target_os = "linux")]
    {
        match detect_backend() {
            Backend::X11 => run_x11(
                scene_id,
                opts,
                doom_enabled,
                settings,
                idle_cycle,
                capture_keys,
            ),
            Backend::Wayland => run_wayland(
                scene_id,
                opts,
                doom_enabled,
                settings,
                idle_cycle,
                capture_keys,
            ),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        );
        Err(ShellError::Unsupported)
    }
}

fn make_scene(
    scene_id: &str,
    doom_enabled: bool,
    columns: usize,
    rows: usize,
    settings: &[(String, f64)],
) -> Result<Box<dyn aa_core::Scene + Send>, String> {
    if scene_id == "doom" {
        if !doom_enabled {
            return Err("DOOM is opt-in. Enable the DOOM scene before selecting it.".into());
        }
        let scaling = aa_doom::scaling_for_grid(columns, rows);
        return Ok(Box::new(aa_doom::DoomScene::new(scaling)));
    }
    let mut scene =
        aa_core::scenes::make(scene_id).ok_or_else(|| format!("unknown scene '{scene_id}'"))?;
    for (id, value) in settings {
        scene.apply_setting(id, *value);
    }
    Ok(scene)
}

#[cfg(target_os = "linux")]
fn run_x11(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
    idle_cycle: bool,
    capture_keys: bool,
) -> Result<(), ShellError> {
    #[cfg(feature = "x11")]
    {
        x11::run(
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        )
        .map_err(ShellError::Backend)
    }
    #[cfg(not(feature = "x11"))]
    {
        let _ = (
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        );
        Err(ShellError::BackendNotCompiled(Backend::X11))
    }
}

#[cfg(target_os = "linux")]
fn run_wayland(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
    idle_cycle: bool,
    capture_keys: bool,
) -> Result<(), ShellError> {
    #[cfg(feature = "wayland")]
    {
        wayland::run(
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        )
        .map_err(ShellError::Backend)
    }
    #[cfg(not(feature = "wayland"))]
    {
        let _ = (
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        );
        Err(ShellError::BackendNotCompiled(Backend::Wayland))
    }
}

/// Swizzle an [`aa_render`] RGBA buffer into the BGRX byte order both X11
/// (ZPixmap, 32bpp LSBFirst) and Wayland (`Xrgb8888`) expect.
#[cfg(target_os = "linux")]
fn rgba_to_bgrx(pixels: &[u8], out: &mut Vec<u8>) {
    out.resize(pixels.len(), 0);
    for (dst, src) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(pixels.as_chunks::<4>().0.iter())
    {
        dst[0] = src[2]; // B
        dst[1] = src[1]; // G
        dst[2] = src[0]; // R
        dst[3] = 0; // X
    }
}

#[cfg(target_os = "linux")]
fn rgba_to_bgra(pixels: &[u8], out: &mut Vec<u8>) {
    out.resize(pixels.len(), 0);
    for (dst, src) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(pixels.as_chunks::<4>().0.iter())
    {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
    }
}

#[cfg(all(target_os = "linux", feature = "x11"))]
mod x11 {
    use super::rgba_to_bgra;
    use aa_render::{render_to_size, RenderOptions};
    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Instant;
    use x11rb::connection::{Connection, RequestConnection};
    use x11rb::protocol::record::{
        self, ConnectionExt as RecordConnectionExt, HType, Range, Range8, CS,
    };
    use x11rb::protocol::screensaver::ConnectionExt as ScreenSaverConnectionExt;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, CreateGCAux, ImageFormat, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    pub fn run(
        scene_id: &str,
        opts: RenderOptions,
        doom_enabled: bool,
        settings: &[(String, f64)],
        idle_cycle: bool,
        capture_keys: bool,
    ) -> Result<(), String> {
        if !opts.opaque_background {
            return Err("The X11 root-pixmap backend cannot blend a transparent background. Use an opaque background for this session.".into());
        }
        let (conn, screen_num) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let depth = screen.root_depth;
        let (w, h) = (screen.width_in_pixels, screen.height_in_pixels);
        let cols = (w as u32 / opts.cell_w.max(1)).max(1) as usize;
        let rows = (h as u32 / opts.cell_h.max(1)).max(1) as usize;
        let mut active_scene = scene_id.to_owned();
        let mut scene = start_scene(&active_scene, &opts, doom_enabled, settings, cols, rows)?;
        let scenes = if doom_enabled {
            vec!["donut", "helix", "matrix", "pipes", "life", "doom"]
        } else {
            vec!["donut", "helix", "matrix", "pipes", "life"]
        };
        let mut cycler = aa_core::idle_cycle::IdleCycle::default();
        let key_events = if capture_keys && doom_enabled {
            Some(observe_global_keys()?)
        } else {
            None
        };

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

        let start = Instant::now();
        let mut next_idle_check = Instant::now();
        let mut idle_seconds = 0.0;
        let mut bgrx: Vec<u8> = Vec::new();
        loop {
            let now = start.elapsed().as_secs_f64();
            if idle_cycle && Instant::now() >= next_idle_check {
                let idle_ms = conn
                    .screensaver_query_info(root)
                    .map_err(|e| e.to_string())?
                    .reply()
                    .map_err(|e| e.to_string())?
                    .ms_since_user_input;
                idle_seconds = f64::from(idle_ms) / 1000.0;
                next_idle_check = Instant::now() + std::time::Duration::from_secs(5);
            }
            if idle_cycle {
                if let Some(next) = cycler.update(&active_scene, idle_seconds, now, &scenes) {
                    scene.stop();
                    scene = start_scene(&next, &opts, doom_enabled, settings, cols, rows)?;
                    active_scene = next;
                }
            }
            if let Some(events) = &key_events {
                for bytes in events.try_iter() {
                    if scene.is_interactive() {
                        scene.send_key(&bytes);
                    }
                }
            }
            let frame = scene.frame(now);
            let buf = render_to_size(&frame, &opts, u32::from(w), u32::from(h));
            rgba_to_bgra(&buf.pixels, &mut bgrx);

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

    fn start_scene(
        id: &str,
        opts: &RenderOptions,
        doom_enabled: bool,
        settings: &[(String, f64)],
        cols: usize,
        rows: usize,
    ) -> Result<Box<dyn aa_core::Scene + Send>, String> {
        let mut scene = super::make_scene(id, doom_enabled, cols, rows, settings)?;
        scene.apply_base_color(opts.theme.text);
        scene.set_grid(cols, rows);
        scene.start();
        Ok(scene)
    }

    fn observe_global_keys() -> Result<Receiver<Vec<u8>>, String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("ascii-arcade-key-observer".into())
            .spawn(move || {
                let Ok((conn, _)) = x11rb::connect(None) else {
                    return;
                };
                if conn
                    .extension_information(record::X11_EXTENSION_NAME)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    return;
                }
                let setup = conn.setup();
                let count = setup
                    .max_keycode
                    .saturating_sub(setup.min_keycode)
                    .saturating_add(1);
                let mapping = match conn.get_keyboard_mapping(setup.min_keycode, count) {
                    Ok(cookie) => match cookie.reply() {
                        Ok(mapping) => mapping,
                        Err(_) => return,
                    },
                    Err(_) => return,
                };
                let per_keycode = usize::from(mapping.keysyms_per_keycode);
                if per_keycode == 0 {
                    return;
                }
                let keysyms: HashMap<u8, u32> = (setup.min_keycode..=setup.max_keycode)
                    .filter_map(|keycode| {
                        let offset = usize::from(keycode - setup.min_keycode) * per_keycode;
                        mapping
                            .keysyms
                            .get(offset)
                            .copied()
                            .map(|keysym| (keycode, keysym))
                    })
                    .collect();
                let Ok(context) = conn.generate_id() else {
                    return;
                };
                let range = Range {
                    delivered_events: Range8 { first: 2, last: 2 },
                    ..Default::default()
                };
                let context_created = match conn.record_create_context(
                    context,
                    HType::FROM_SERVER_TIME.into(),
                    &[u32::from(CS::ALL_CLIENTS)],
                    &[range],
                ) {
                    Ok(cookie) => cookie.check().is_ok(),
                    Err(_) => false,
                };
                if !context_created {
                    return;
                }
                let Ok(replies) = conn.record_enable_context(context) else {
                    return;
                };
                for reply in replies {
                    let Ok(reply) = reply else { break };
                    if reply.category != 0 || reply.data.len() < 32 || reply.data[0] & 0x7f != 2 {
                        continue;
                    }
                    let Some(keysym) = keysyms.get(&reply.data[1]) else {
                        continue;
                    };
                    if let Some(bytes) = key_bytes(*keysym) {
                        if sender.send(bytes).is_err() {
                            break;
                        }
                    }
                }
            })
            .map_err(|error| format!("could not start DOOM key capture: {error}"))?;
        Ok(receiver)
    }

    fn key_bytes(keysym: u32) -> Option<Vec<u8>> {
        let sequence: &[u8] = match keysym {
            0xff51 => b"\x1b[D",
            0xff52 => b"\x1b[A",
            0xff53 => b"\x1b[C",
            0xff54 => b"\x1b[B",
            0xff0d | 0xff8d => b"\n",
            0xff1b => b"\x1b",
            0x20 => b" ",
            0x2c => b",",
            0x2e => b".",
            0x5d | 0x7d => b"]",
            0x21..=0x7e => {
                return Some(
                    vec![keysym as u8]
                        .into_iter()
                        .map(|ch| ch.to_ascii_lowercase())
                        .collect(),
                )
            }
            _ => return None,
        };
        Some(sequence.to_vec())
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

    use super::{rgba_to_bgra, rgba_to_bgrx};
    use aa_render::{render_to_size, RenderOptions};
    use std::time::Instant;

    use smithay_client_toolkit::reexports::client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_shm, wl_surface},
        Connection, QueueHandle,
    };
    use smithay_client_toolkit::{
        compositor::{CompositorHandler, CompositorState, FrameCallbackData},
        delegate_dispatch2, delegate_registry,
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

    pub fn run(
        scene_id: &str,
        opts: RenderOptions,
        doom_enabled: bool,
        settings: &[(String, f64)],
        idle_cycle: bool,
        capture_keys: bool,
    ) -> Result<(), String> {
        if idle_cycle || (capture_keys && doom_enabled) {
            return Err("This Wayland compositor does not expose system-wide idle and key events to wallpaper clients. GNOME Shell provides both through the ASCII Arcade extension; X11 provides them through its session input APIs.".into());
        }
        let mut scene = super::make_scene(scene_id, doom_enabled, 320, 100, settings)?;
        // Colour scenes (Matrix, …) key their palette off the theme colour.
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

            let format = if self.opts.opaque_background {
                wl_shm::Format::Xrgb8888
            } else {
                wl_shm::Format::Argb8888
            };
            let (buffer, canvas) = match self.pool.create_buffer(w as i32, h as i32, stride, format)
            {
                Ok(b) => b,
                Err(_) => return,
            };

            let frame = self.scene.frame(self.start.elapsed().as_secs_f64());
            let buf = render_to_size(&frame, &self.opts, w, h);
            if self.opts.opaque_background {
                rgba_to_bgrx(&buf.pixels, &mut self.scratch);
            } else {
                rgba_to_bgra(&buf.pixels, &mut self.scratch);
            }

            // Fill the theme background, then copy the rendered region (which can
            // be a few pixels smaller than the surface due to integer cell sizing).
            let bg = self.opts.theme.background;
            for px in canvas.as_chunks_mut::<4>().0 {
                if self.opts.opaque_background {
                    px[0] = bg.b;
                    px[1] = bg.g;
                    px[2] = bg.r;
                    px[3] = 255;
                } else {
                    px.copy_from_slice(&[0, 0, 0, 0]);
                }
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
            surface.frame(qh, FrameCallbackData(surface.clone()));
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

    delegate_registry!(State);
    delegate_dispatch2!(State);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_detection_is_total() {
        let _ = detect_backend();
    }

    #[test]
    fn detect_backend_wayland_and_x11() {
        // Test with WAYLAND_DISPLAY set
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert_eq!(detect_backend(), Backend::Wayland);

        // Test with WAYLAND_DISPLAY unset
        std::env::remove_var("WAYLAND_DISPLAY");
        assert_eq!(detect_backend(), Backend::X11);
    }

    #[test]
    fn shell_error_display_and_error_impl() {
        let unsupp = ShellError::Unsupported;
        assert_eq!(format!("{unsupp}"), "the Linux shell only runs on Linux");
        assert!((&unsupp as &dyn std::error::Error).source().is_none());

        let not_x11 = ShellError::BackendNotCompiled(Backend::X11);
        assert_eq!(
            format!("{not_x11}"),
            "X11 backend not compiled in (enable its cargo feature)"
        );

        let not_wl = ShellError::BackendNotCompiled(Backend::Wayland);
        assert_eq!(
            format!("{not_wl}"),
            "Wayland backend not compiled in (enable its cargo feature)"
        );

        #[cfg(target_os = "linux")]
        {
            let backend_err = ShellError::Backend("test failure".into());
            assert_eq!(format!("{backend_err}"), "backend error: test failure");
        }
    }

    #[test]
    fn run_returns_expected_error_on_unknown_scene_or_display() {
        let opts = RenderOptions::default();
        // Running on non-existent display or invalid scene should return error gracefully without panicking
        let res = run("non_existent_scene_xyz", opts);
        assert!(res.is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rgba_to_bgrx_swizzles_correctly() {
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
        let mut bgrx = Vec::new();
        rgba_to_bgrx(&rgba, &mut bgrx);
        assert_eq!(bgrx, vec![30, 20, 10, 0, 60, 50, 40, 0]);
    }

    #[test]
    fn autostart_lifecycle_in_tempdir() {
        let temp_home =
            std::env::temp_dir().join(format!("aa-linux-autostart-{}", std::process::id()));
        std::fs::create_dir_all(&temp_home).unwrap();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &temp_home);

        #[cfg(target_os = "linux")]
        {
            assert!(!autostart::is_installed());
            assert!(autostart::install("matrix", "amber").is_ok());
            assert!(autostart::is_installed());

            // Read the installed desktop file
            let desktop_file = temp_home.join(".config/autostart/ascii-arcade.desktop");
            let content = std::fs::read_to_string(&desktop_file).unwrap();
            assert!(content.contains("matrix amber"));
            assert!(content.contains("[Desktop Entry]"));

            assert!(autostart::remove().is_ok());
            assert!(!autostart::is_installed());
            // Double removal should be Ok
            assert!(autostart::remove().is_ok());
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert!(!autostart::is_installed());
            assert!(autostart::install("matrix", "amber").is_err());
            assert!(autostart::remove().is_err());
        }

        if let Some(h) = old_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
        std::fs::remove_dir_all(&temp_home).ok();
    }
}

/// Manage the XDG autostart `.desktop` entry that tells the desktop session to
/// launch `aa-linux` at login.
pub mod autostart {
    /// Install `~/.config/autostart/ascii-arcade.desktop` so `aa-linux` starts
    /// with the user's graphical session. `scene` and `theme` become CLI args.
    pub fn install(scene: &str, theme: &str) -> Result<(), String> {
        install_with_doom(scene, theme, false)
    }

    pub fn install_with_doom(scene: &str, theme: &str, doom_enabled: bool) -> Result<(), String> {
        let args = if doom_enabled {
            vec!["--enable-doom".to_owned()]
        } else {
            Vec::new()
        };
        install_with_args(scene, theme, &args)
    }

    pub fn install_with_args(scene: &str, theme: &str, args: &[String]) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            imp::install(scene, theme, args)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (scene, theme, args);
            Err("autostart is only supported on Linux".into())
        }
    }

    /// Remove the `~/.config/autostart/ascii-arcade.desktop` entry, if present.
    pub fn remove() -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            imp::remove()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("autostart is only supported on Linux".into())
        }
    }

    /// Returns `true` if the autostart `.desktop` file currently exists.
    pub fn is_installed() -> bool {
        #[cfg(target_os = "linux")]
        {
            imp::is_installed()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    #[cfg(target_os = "linux")]
    mod imp {
        use std::path::PathBuf;

        fn desktop_path() -> Option<PathBuf> {
            let home = std::env::var_os("HOME")?;
            Some(PathBuf::from(home).join(".config/autostart/ascii-arcade.desktop"))
        }

        pub fn install(scene: &str, theme: &str, args: &[String]) -> Result<(), String> {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let path = desktop_path().ok_or("could not determine $HOME")?;
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            let extra = args.join(" ");
            let suffix = if extra.is_empty() {
                String::new()
            } else {
                format!(" {extra}")
            };
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=ASCII Arcade\n\
                 Comment=Live ASCII wallpaper\n\
                 Exec={} {} {}{}\n\
                 X-GNOME-Autostart-enabled=true\n\
                 Hidden=false\n\
                 NoDisplay=false\n",
                exe.display(),
                scene,
                theme,
                suffix,
            );
            std::fs::write(&path, content).map_err(|e| e.to_string())?;
            println!("autostart installed: {}", path.display());
            Ok(())
        }

        pub fn remove() -> Result<(), String> {
            let path = desktop_path().ok_or("could not determine $HOME")?;
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
                println!("autostart removed: {}", path.display());
            }
            Ok(())
        }

        pub fn is_installed() -> bool {
            desktop_path().map(|p| p.exists()).unwrap_or(false)
        }
    }
}
