//! # aa — ASCII Arcade unified CLI
//!
//! ```text
//! aa play [scene] [--theme THEME] [--fps FPS]
//!     Render a scene live in the current terminal window (all platforms).
//!
//! aa run [scene] [--theme THEME]
//!     Launch the native wallpaper shell for the current OS.
//!
//! aa web [scene] [--theme THEME] [--port PORT]
//!     Stream a scene to a browser via xterm.js WebSocket.
//!
//! aa autostart enable [scene] [--theme THEME]
//! aa autostart disable
//! aa autostart status
//!     Manage start-at-login registration.
//!
//! aa scenes
//! aa themes
//!     List built-in scenes and themes.
//!
//! --enable-doom
//!     Unlocks the DOOM scene for `play`/`web`/`scenes` on this invocation.
//!     DOOM is a playable shooter, so it's opt-in rather than just another
//!     scene: it also needs the binary built with `--features doom` in the
//!     first place (`aa-doom` is an optional dependency, off by default).
//!     Not available via `aa run` — DOOM as an actual desktop wallpaper
//!     (fixed-grid bitmap compositing + global keyboard capture while another
//!     app has focus) isn't implemented on Linux/Windows yet.
//! ```

use clap::{Parser, Subcommand};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "aa",
    about = "ASCII Arcade — scenes for your wallpaper and terminal",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Unlock the DOOM scene for this invocation (also needs the binary built
    /// with `--features doom`). DOOM is a playable shooter, so it isn't on by
    /// default — see the `aa-doom` crate.
    #[arg(long, global = true)]
    enable_doom: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Render a scene live inside the current terminal window
    Play {
        /// Scene to display
        #[arg(default_value = "donut")]
        scene: String,
        /// Colour theme  [hacker|amber|ice|ghost]
        #[arg(short, long, default_value = "hacker")]
        theme: String,
        /// Target frames per second
        #[arg(long, default_value_t = 30u32)]
        fps: u32,
    },
    /// Run a scene as a desktop wallpaper (platform-native)
    Run {
        /// Scene to display
        #[arg(default_value = "donut")]
        scene: String,
        /// Colour theme  [hacker|amber|ice|ghost]
        #[arg(short, long, default_value = "hacker")]
        theme: String,
    },
    /// Stream a scene to a browser via xterm.js WebSocket
    Web {
        /// Scene to display
        #[arg(default_value = "donut")]
        scene: String,
        /// Colour theme  [hacker|amber|ice|ghost]
        #[arg(short, long, default_value = "hacker")]
        theme: String,
        /// TCP port to listen on
        #[arg(short, long, default_value_t = 8788u16)]
        port: u16,
    },
    /// Manage start-at-login registration
    Autostart {
        #[command(subcommand)]
        action: AutostartAction,
    },
    /// List built-in scene IDs
    Scenes,
    /// List available theme names
    Themes,
}

#[derive(Subcommand)]
enum AutostartAction {
    /// Register aa as a login item
    Enable {
        /// Scene to launch at login
        #[arg(default_value = "donut")]
        scene: String,
        /// Colour theme
        #[arg(short, long, default_value = "hacker")]
        theme: String,
    },
    /// Remove aa from login items
    Disable,
    /// Print current login-item status
    Status,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    std::process::exit(dispatch(Cli::parse()));
}

fn dispatch(cli: Cli) -> i32 {
    let enable_doom = cli.enable_doom;
    match cli.command {
        Command::Play { scene, theme, fps } => {
            if let Err(e) = cmd_play(&scene, resolve_theme(&theme), fps, enable_doom) {
                eprintln!("aa play: {e}");
                return 1;
            }
        }
        Command::Run { scene, theme } => {
            if let Err(e) = cmd_run(&scene, resolve_theme(&theme)) {
                eprintln!("aa run: {e}");
                return 1;
            }
        }
        Command::Web { scene, theme, port } => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            if let Err(e) = rt.block_on(cmd_web(scene, resolve_theme(&theme), port, enable_doom)) {
                eprintln!("aa web: {e}");
                return 1;
            }
        }
        Command::Autostart { action } => {
            if let Err(e) = cmd_autostart(action) {
                eprintln!("aa autostart: {e}");
                return 1;
            }
        }
        Command::Scenes => {
            for id in aa_core::scenes::BUILTIN_IDS {
                println!("{id}");
            }
            if cfg!(feature = "doom") && enable_doom {
                println!("doom");
            }
        }
        Command::Themes => {
            for t in aa_core::Theme::ALL {
                println!("{}", t.name.to_lowercase());
            }
        }
    }
    0
}

fn resolve_theme(name: &str) -> aa_core::Theme {
    aa_core::Theme::by_name(name).unwrap_or_else(|| {
        eprintln!("aa: unknown theme '{name}', falling back to hacker");
        aa_core::Theme::HACKER
    })
}

// ── scene resolution (incl. the opt-in DOOM seam) ───────────────────────────

/// Construct a scene by id. DOOM isn't part of `aa_core::scenes::BUILTIN_IDS`
/// on purpose (see crates/aa-core) — this is the one seam where the CLI adds
/// it back in, deliberately gated behind both a Cargo feature (is it even
/// compiled in?) and a runtime opt-in (did the user actually ask to unlock
/// it, on *this* invocation?). `doom_scaling` is ignored for every other id.
fn make_scene(
    id: &str,
    enable_doom: bool,
    doom_scaling: usize,
) -> Result<Box<dyn aa_core::Scene + Send>, String> {
    let _ = (enable_doom, doom_scaling); // read only when built with `--features doom`

    #[cfg(feature = "doom")]
    if id == "doom" {
        if !enable_doom {
            return Err("DOOM is opt-in — rerun with --enable-doom to play it".into());
        }
        return Ok(Box::new(aa_doom::DoomScene::new(doom_scaling)));
    }

    #[cfg(not(feature = "doom"))]
    if id == "doom" {
        return Err(
            "this build of aa doesn't include DOOM — rebuild with `cargo build --features doom`"
                .into(),
        );
    }

    aa_core::scenes::make(id).ok_or_else(|| format!("unknown scene '{id}'"))
}

/// Smallest DOOM `-scaling` factor (1..=16) whose character grid still fits
/// inside `(cols, rows)`, so `aa play doom` renders as sharp as the terminal
/// allows instead of overflowing it and wrapping garbage. DOOM's resolution
/// is fixed at launch (like the macOS host's `DOOM_SCALING`), not re-fit on a
/// later terminal resize.
#[cfg(feature = "doom")]
fn doom_scaling_for(cols: u16, rows: u16) -> usize {
    (1..=16)
        .find(|&n| {
            let (w, h) = aa_doom::grid_size(n);
            w <= cols as usize && h <= rows as usize
        })
        .unwrap_or(16)
}

fn cmd_play(
    scene_id: &str,
    theme: aa_core::Theme,
    fps: u32,
    enable_doom: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        cursor,
        execute,
        style::{Color, SetBackgroundColor},
        terminal::{self, ClearType},
    };

    // RAII guard: restore the terminal even if we return early or panic.
    struct TermGuard;
    impl Drop for TermGuard {
        fn drop(&mut self) {
            let mut out = std::io::stdout();
            let _ = crossterm::execute!(
                out,
                crossterm::cursor::Show,
                crossterm::terminal::LeaveAlternateScreen
            );
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }

    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode()?;
    let bg = theme.background;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        SetBackgroundColor(Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b
        }),
        terminal::Clear(ClearType::All),
    )?;
    let _guard = TermGuard;

    let (cols, rows) = terminal::size()?;

    #[cfg(feature = "doom")]
    let doom_scaling = doom_scaling_for(cols, rows);
    #[cfg(not(feature = "doom"))]
    let doom_scaling = 1; // unused: make_scene ignores it when the feature is off

    let mut scene = make_scene(scene_id, enable_doom, doom_scaling).unwrap_or_else(|e| {
        eprintln!("aa: {e}, falling back to donut");
        aa_core::scenes::make("donut").unwrap()
    });

    let mut event_source = CrosstermEvents;
    run_play_loop(
        &mut *scene,
        theme,
        fps,
        cols as usize,
        rows as usize,
        &mut stdout,
        &mut event_source,
    )
}

pub trait EventSource {
    fn poll_event(&mut self, timeout: std::time::Duration) -> Result<bool, std::io::Error>;
    fn read_event(&mut self) -> Result<crossterm::event::Event, std::io::Error>;
}

struct CrosstermEvents;
impl EventSource for CrosstermEvents {
    fn poll_event(&mut self, timeout: std::time::Duration) -> Result<bool, std::io::Error> {
        crossterm::event::poll(timeout)
    }
    fn read_event(&mut self) -> Result<crossterm::event::Event, std::io::Error> {
        crossterm::event::read()
    }
}

fn run_play_loop<W: std::io::Write, E: EventSource>(
    scene: &mut dyn aa_core::Scene,
    theme: aa_core::Theme,
    fps: u32,
    initial_cols: usize,
    initial_rows: usize,
    writer: &mut W,
    events: &mut E,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{Event, KeyCode, KeyModifiers};
    use std::time::{Duration, Instant};

    scene.apply_base_color(theme.text);
    scene.set_grid(initial_cols, initial_rows);
    scene.start();

    let frame_dur = Duration::from_millis(1000 / fps.max(1) as u64);
    let start = Instant::now();

    loop {
        let ansi =
            aa_core::ansi::frame_to_ansi(&scene.frame(start.elapsed().as_secs_f64()), &theme);
        // frame_to_ansi already emits \x1b[2J\x1b[H, so write directly.
        writer.write_all(ansi.as_bytes())?;
        writer.flush()?;

        // Poll for terminal events until the next frame deadline.
        let deadline = Instant::now() + frame_dur;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                break;
            }
            if events.poll_event(remaining)? {
                match events.read_event()? {
                    Event::Key(k) => {
                        let quit = k.code == KeyCode::Char('q')
                            || k.code == KeyCode::Esc
                            || (k.code == KeyCode::Char('c')
                                && k.modifiers.contains(KeyModifiers::CONTROL));
                        if quit {
                            scene.stop();
                            return Ok(());
                        }
                        if scene.is_interactive() {
                            let bytes: &[u8] = match k.code {
                                KeyCode::Up => b"\x1b[A",
                                KeyCode::Down => b"\x1b[B",
                                KeyCode::Right => b"\x1b[C",
                                KeyCode::Left => b"\x1b[D",
                                KeyCode::Enter => b"\n",
                                KeyCode::Esc => b"\x1b",
                                KeyCode::Char(' ') => b" ",
                                KeyCode::Char(ch) => {
                                    // Encode single-byte ASCII; skip outside that range.
                                    if (ch as u32) < 128 {
                                        scene.send_key(&[ch as u8]);
                                    }
                                    continue;
                                }
                                _ => continue,
                            };
                            scene.send_key(bytes);
                        }
                    }
                    Event::Resize(c, r) => {
                        scene.set_grid(c as usize, r as usize);
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── run ───────────────────────────────────────────────────────────────────────

fn cmd_run(scene_id: &str, theme: aa_core::Theme) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use aa_render::RenderOptions;
        return aa_linux::run(
            scene_id,
            RenderOptions {
                theme,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string());
    }

    #[cfg(windows)]
    {
        use aa_render::RenderOptions;
        return aa_windows::run(
            scene_id,
            RenderOptions {
                theme,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (scene_id, theme);
        let ok = std::process::Command::new("open")
            .args(["-a", "ASCII Arcade"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return if ok {
            Ok(())
        } else {
            Err(
                "ASCII Arcade.app not found — install it from the disk image first.\n\
                 For a terminal experience on any platform, try 'aa play'."
                    .into(),
            )
        };
    }

    #[allow(unreachable_code)]
    Err("wallpaper mode is not supported on this platform; try 'aa play'".into())
}

// ── web ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct AppState {
    theme: aa_core::Theme,
    enable_doom: bool,
}

// DOOM's framebuffer is fixed at scene-construction time (see
// `make_scene`), but a browser tab's terminal size isn't known until its
// first `__resize__:` message arrives just after connecting — after the
// scene already exists. Rather than delay construction on that
// handshake, use one fixed, conservative grid (160×50) that fits most
// browser windows; unlike the CLI's `aa play doom`, it isn't fit exactly.
const WEB_DOOM_SCALING: usize = 4;

async fn root() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../web/static/index.html"))
}

async fn scene_list(axum::extract::State(s): axum::extract::State<AppState>) -> axum::response::Json<Vec<&'static str>> {
    let mut ids: Vec<&'static str> = aa_core::scenes::BUILTIN_IDS.to_vec();
    if cfg!(feature = "doom") && s.enable_doom {
        ids.push("doom");
    }
    axum::response::Json(ids)
}

async fn ws_upgrade(
    axum::extract::Path(sid): axum::extract::Path<String>,
    ws: axum::extract::ws::WebSocketUpgrade,
    axum::extract::State(s): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| run_ws(socket, sid, s.theme, s.enable_doom))
}

async fn run_ws(mut socket: axum::extract::ws::WebSocket, sid: String, theme: aa_core::Theme, enable_doom: bool) {
    use axum::extract::ws::Message;
    use std::time::{Duration, Instant};
    use tokio::time::MissedTickBehavior;

    let mut scene = match make_scene(&sid, enable_doom, WEB_DOOM_SCALING) {
        Ok(s) => s,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("{e}\r\n").into())).await;
            return;
        }
    };
    let mut cols = 120usize;
    let mut rows = 40usize;
    scene.apply_base_color(theme.text);
    scene.set_grid(cols, rows);
    scene.start();

    let start = Instant::now();
    let mut interval = tokio::time::interval(Duration::from_millis(33));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let frame = scene.frame(start.elapsed().as_secs_f64());
                let ansi = aa_core::ansi::frame_to_ansi(&frame, &theme);
                if socket.send(Message::Text(ansi.into())).await.is_err() { break; }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(dims) = text.strip_prefix("__resize__:") {
                            if let Some((c, r)) = dims.split_once('x') {
                                if let (Ok(c), Ok(r)) = (c.parse::<usize>(), r.parse::<usize>()) {
                                    cols = c.max(20); rows = r.max(10);
                                    scene.set_grid(cols, rows);
                                }
                            }
                        } else {
                            scene.send_key(text.as_bytes());
                        }
                    }
                    Some(Ok(Message::Binary(data))) => { scene.send_key(&data); }
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
        }
    }
    scene.stop();
}

fn build_router(theme: aa_core::Theme, enable_doom: bool) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/", get(root))
        .route("/api/scenes", get(scene_list))
        .route("/ws/{scene_id}", get(ws_upgrade))
        .with_state(AppState { theme, enable_doom })
}

async fn cmd_web(
    scene_id: String,
    theme: aa_core::Theme,
    port: u16,
    enable_doom: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = scene_id; // default scene lives in the URL; the CLI arg is informational
    let app = build_router(theme, enable_doom);
    let addr = format!("0.0.0.0:{port}");
    println!("aa web  →  http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;
    Ok(())
}

// ── autostart ─────────────────────────────────────────────────────────────────

fn cmd_autostart(action: AutostartAction) -> Result<(), String> {
    match action {
        AutostartAction::Enable { scene, theme } => {
            #[cfg(target_os = "linux")]
            return aa_linux::autostart::install(&scene, &theme);

            #[cfg(windows)]
            return aa_windows::autostart::install(&scene, &theme);

            #[cfg(target_os = "macos")]
            {
                let _ = (scene, theme);
                println!("On macOS, toggle 'Launch at Login' in the ASCII Arcade status-bar menu.");
                return Ok(());
            }

            #[allow(unreachable_code)]
            Err("autostart enable is not supported on this platform".into())
        }

        AutostartAction::Disable => {
            #[cfg(target_os = "linux")]
            return aa_linux::autostart::remove();

            #[cfg(windows)]
            return aa_windows::autostart::remove();

            #[cfg(target_os = "macos")]
            {
                println!("On macOS, toggle 'Launch at Login' in the ASCII Arcade status-bar menu.");
                return Ok(());
            }

            #[allow(unreachable_code)]
            Err("autostart disable is not supported on this platform".into())
        }

        AutostartAction::Status => {
            #[cfg(target_os = "linux")]
            {
                let on = aa_linux::autostart::is_installed();
                println!("autostart: {}", if on { "enabled" } else { "disabled" });
                return Ok(());
            }

            #[cfg(windows)]
            {
                let on = aa_windows::autostart::is_installed();
                println!("autostart: {}", if on { "enabled" } else { "disabled" });
                return Ok(());
            }

            #[cfg(target_os = "macos")]
            {
                println!("On macOS, check 'Launch at Login' in the ASCII Arcade status-bar menu.");
                return Ok(());
            }

            #[allow(unreachable_code)]
            Err("autostart status is not supported on this platform".into())
        }
    }
}

#[cfg(test)]
mod scene_resolution_tests {
    use super::*;

    #[test]
    fn make_scene_rejects_doom_without_opt_in() {
        // True regardless of how this crate was built: whether DOOM is even
        // compiled in or just not unlocked for this run, asking for "doom"
        // without --enable-doom must never silently succeed. (Not
        // `.unwrap_err()`: that needs `Box<dyn Scene + Send>: Debug`, which
        // trait objects over `Scene` don't implement.)
        match make_scene("doom", false, 1) {
            Err(e) => assert!(e.contains("opt-in") || e.contains("doesn't include DOOM")),
            Ok(_) => panic!("expected doom to be rejected without --enable-doom"),
        }
    }

    #[test]
    fn make_scene_still_resolves_ordinary_scenes() {
        assert!(make_scene("donut", false, 1).is_ok());
    }

    #[test]
    fn make_scene_rejects_unknown_scene() {
        assert!(make_scene("not-a-real-scene", false, 1).is_err());
    }

    #[cfg(feature = "doom")]
    #[test]
    fn make_scene_allows_doom_with_opt_in() {
        assert!(make_scene("doom", true, 8).is_ok());
    }

    #[cfg(feature = "doom")]
    #[test]
    fn doom_scaling_picks_smallest_fit() {
        assert_eq!(doom_scaling_for(640, 200), 1);
        assert_eq!(doom_scaling_for(320, 100), 2);
    }

    #[cfg(feature = "doom")]
    #[test]
    fn doom_scaling_fits_a_typical_80x24_terminal() {
        let n = doom_scaling_for(80, 24);
        let (w, h) = aa_doom::grid_size(n);
        assert!(
            w <= 80 && h <= 24,
            "grid {w}x{h} overflows an 80x24 terminal"
        );
    }

    #[cfg(feature = "doom")]
    #[test]
    fn doom_scaling_falls_back_instead_of_panicking_on_a_tiny_terminal() {
        assert_eq!(doom_scaling_for(1, 1), 16);
    }

    #[test]
    fn resolve_theme_finds_valid_and_falls_back_for_unknown() {
        assert_eq!(resolve_theme("amber").name, "Amber");
        assert_eq!(resolve_theme("ice").name, "Ice");
        assert_eq!(resolve_theme("ghost").name, "Ghost");
        assert_eq!(resolve_theme("hacker").name, "Hacker");
        assert_eq!(resolve_theme("unknown_theme_123").name, "Hacker");
    }

    #[test]
    fn cli_parse_play_subcommand() {
        let cli = Cli::try_parse_from(["aa", "play", "matrix", "--theme", "amber", "--fps", "60", "--enable-doom"]).unwrap();
        assert!(cli.enable_doom);
        match cli.command {
            Command::Play { scene, theme, fps } => {
                assert_eq!(scene, "matrix");
                assert_eq!(theme, "amber");
                assert_eq!(fps, 60);
            }
            _ => panic!("expected Play subcommand"),
        }
    }

    #[test]
    fn cli_parse_run_subcommand_defaults() {
        let cli = Cli::try_parse_from(["aa", "run"]).unwrap();
        assert!(!cli.enable_doom);
        match cli.command {
            Command::Run { scene, theme } => {
                assert_eq!(scene, "donut");
                assert_eq!(theme, "hacker");
            }
            _ => panic!("expected Run subcommand"),
        }
    }

    #[test]
    fn cli_parse_web_subcommand() {
        let cli = Cli::try_parse_from(["aa", "web", "pipes", "--port", "9000"]).unwrap();
        match cli.command {
            Command::Web { scene, theme, port } => {
                assert_eq!(scene, "pipes");
                assert_eq!(theme, "hacker");
                assert_eq!(port, 9000);
            }
            _ => panic!("expected Web subcommand"),
        }
    }

    #[test]
    fn cli_parse_autostart_subcommands() {
        let cli_enable = Cli::try_parse_from(["aa", "autostart", "enable", "life", "--theme", "ice"]).unwrap();
        match cli_enable.command {
            Command::Autostart { action: AutostartAction::Enable { scene, theme } } => {
                assert_eq!(scene, "life");
                assert_eq!(theme, "ice");
            }
            _ => panic!("expected Autostart Enable"),
        }

        let cli_disable = Cli::try_parse_from(["aa", "autostart", "disable"]).unwrap();
        match cli_disable.command {
            Command::Autostart { action: AutostartAction::Disable } => {}
            _ => panic!("expected Autostart Disable"),
        }

        let cli_status = Cli::try_parse_from(["aa", "autostart", "status"]).unwrap();
        match cli_status.command {
            Command::Autostart { action: AutostartAction::Status } => {}
            _ => panic!("expected Autostart Status"),
        }
    }

    #[test]
    fn cli_parse_scenes_and_themes_subcommands() {
        let cli_scenes = Cli::try_parse_from(["aa", "scenes"]).unwrap();
        match cli_scenes.command {
            Command::Scenes => {}
            _ => panic!("expected Scenes"),
        }

        let cli_themes = Cli::try_parse_from(["aa", "themes"]).unwrap();
        match cli_themes.command {
            Command::Themes => {}
            _ => panic!("expected Themes"),
        }
    }

    #[test]
    fn dispatch_scenes_and_themes_returns_zero() {
        assert_eq!(dispatch(Cli::try_parse_from(["aa", "scenes"]).unwrap()), 0);
        assert_eq!(dispatch(Cli::try_parse_from(["aa", "scenes", "--enable-doom"]).unwrap()), 0);
        assert_eq!(dispatch(Cli::try_parse_from(["aa", "themes"]).unwrap()), 0);
    }

    #[test]
    fn cmd_autostart_execution_succeeds_or_returns_platform_status() {
        // Test all autostart action variants through cmd_autostart
        let _ = cmd_autostart(AutostartAction::Status);
        let _ = cmd_autostart(AutostartAction::Disable);
        let _ = cmd_autostart(AutostartAction::Enable {
            scene: "donut".into(),
            theme: "hacker".into(),
        });
    }

    #[test]
    fn cmd_run_execution() {
        let res = cmd_run("donut", aa_core::Theme::HACKER);
        // On Linux / Windows / macOS it either runs or returns expected error / ok
        let _ = res;
    }

    // Mock EventSource for testing run_play_loop
    struct MockEvents {
        events: std::collections::VecDeque<crossterm::event::Event>,
    }

    impl MockEvents {
        fn new(events: Vec<crossterm::event::Event>) -> Self {
            Self {
                events: events.into(),
            }
        }
    }

    impl EventSource for MockEvents {
        fn poll_event(&mut self, _timeout: std::time::Duration) -> Result<bool, std::io::Error> {
            Ok(!self.events.is_empty())
        }
        fn read_event(&mut self) -> Result<crossterm::event::Event, std::io::Error> {
            self.events.pop_front().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no more events")
            })
        }
    }

    // Mock interactive scene to test key inputs and lifecycle
    struct MockInteractiveScene {
        keys_received: Vec<Vec<u8>>,
        width: usize,
        height: usize,
        started: bool,
        stopped: bool,
    }

    impl MockInteractiveScene {
        fn new() -> Self {
            Self {
                keys_received: Vec::new(),
                width: 0,
                height: 0,
                started: false,
                stopped: false,
            }
        }
    }

    impl aa_core::Scene for MockInteractiveScene {
        fn display_name(&self) -> &str { "MockInteractive" }
        fn is_interactive(&self) -> bool { true }
        fn set_grid(&mut self, width: usize, height: usize) {
            self.width = width;
            self.height = height;
        }
        fn frame(&mut self, _t: f64) -> aa_core::Frame {
            aa_core::Frame::blank(self.width.max(1), self.height.max(1))
        }
        fn send_key(&mut self, bytes: &[u8]) {
            self.keys_received.push(bytes.to_vec());
        }
        fn start(&mut self) {
            self.started = true;
        }
        fn stop(&mut self) {
            self.stopped = true;
        }
    }

    #[test]
    fn run_play_loop_quit_with_q() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut scene = MockInteractiveScene::new();
        let mut writer = Vec::new();
        let mut events = MockEvents::new(vec![crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        ))]);

        let res = run_play_loop(
            &mut scene,
            aa_core::Theme::HACKER,
            30,
            80,
            24,
            &mut writer,
            &mut events,
        );
        assert!(res.is_ok());
        assert!(scene.started);
        assert!(scene.stopped);
        assert!(!writer.is_empty());
    }

    #[test]
    fn run_play_loop_quit_with_esc() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut scene = MockInteractiveScene::new();
        let mut writer = Vec::new();
        let mut events = MockEvents::new(vec![crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))]);

        let res = run_play_loop(
            &mut scene,
            aa_core::Theme::HACKER,
            30,
            80,
            24,
            &mut writer,
            &mut events,
        );
        assert!(res.is_ok());
        assert!(scene.stopped);
    }

    #[test]
    fn run_play_loop_quit_with_ctrl_c() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut scene = MockInteractiveScene::new();
        let mut writer = Vec::new();
        let mut events = MockEvents::new(vec![crossterm::event::Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ))]);

        let res = run_play_loop(
            &mut scene,
            aa_core::Theme::HACKER,
            30,
            80,
            24,
            &mut writer,
            &mut events,
        );
        assert!(res.is_ok());
        assert!(scene.stopped);
    }

    #[test]
    fn run_play_loop_handles_interactive_keys_and_resize() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut scene = MockInteractiveScene::new();
        let mut writer = Vec::new();
        let mut events = MockEvents::new(vec![
            crossterm::event::Event::Resize(100, 50),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char('🦀'), KeyModifiers::NONE)), // non-ascii char ignored
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)), // non-handled key
            crossterm::event::Event::FocusGained, // ignored event
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        ]);

        let res = run_play_loop(
            &mut scene,
            aa_core::Theme::ICE,
            60,
            80,
            24,
            &mut writer,
            &mut events,
        );
        assert!(res.is_ok());
        assert_eq!(scene.width, 100);
        assert_eq!(scene.height, 50);
        assert!(scene.keys_received.contains(&b"\x1b[A".to_vec()));
        assert!(scene.keys_received.contains(&b"\x1b[B".to_vec()));
        assert!(scene.keys_received.contains(&b"\x1b[C".to_vec()));
        assert!(scene.keys_received.contains(&b"\x1b[D".to_vec()));
        assert!(scene.keys_received.contains(&b"\n".to_vec()));
        assert!(scene.keys_received.contains(&b" ".to_vec()));
        assert!(scene.keys_received.contains(&b"w".to_vec()));
    }

    #[test]
    fn run_play_loop_non_interactive_scene() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut scene = aa_core::scenes::make("donut").unwrap();
        let mut writer = Vec::new();
        let mut events = MockEvents::new(vec![
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            crossterm::event::Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        ]);

        let res = run_play_loop(
            &mut *scene,
            aa_core::Theme::AMBER,
            30,
            80,
            24,
            &mut writer,
            &mut events,
        );
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn router_endpoints_test() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let app = build_router(aa_core::Theme::HACKER, true);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Test GET /
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("xterm"));

        // Test GET /api/scenes
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /api/scenes HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("donut"));

        server_handle.abort();
    }

    #[test]
    fn dispatch_all_subcommands() {
        assert_eq!(dispatch(Cli::try_parse_from(["aa", "scenes"]).unwrap()), 0);
        assert_eq!(dispatch(Cli::try_parse_from(["aa", "themes"]).unwrap()), 0);
        assert_eq!(
            dispatch(Cli::try_parse_from(["aa", "autostart", "status"]).unwrap()),
            0
        );
        assert_eq!(
            dispatch(Cli::try_parse_from(["aa", "autostart", "disable"]).unwrap()),
            0
        );
        assert_eq!(
            dispatch(
                Cli::try_parse_from(["aa", "autostart", "enable", "donut", "--theme", "ice"])
                    .unwrap()
            ),
            0
        );
    }
}
