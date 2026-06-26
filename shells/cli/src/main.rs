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
    match cli.command {
        Command::Play { scene, theme, fps } => {
            if let Err(e) = cmd_play(&scene, resolve_theme(&theme), fps) {
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
            if let Err(e) = rt.block_on(cmd_web(scene, resolve_theme(&theme), port)) {
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

// ── play ──────────────────────────────────────────────────────────────────────

fn cmd_play(scene_id: &str, theme: aa_core::Theme, fps: u32) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        style::{Color, SetBackgroundColor},
        terminal::{self, ClearType},
    };
    use std::io::Write as _;
    use std::time::{Duration, Instant};

    // RAII guard: restore the terminal even if we return early or panic.
    struct TermGuard;
    impl Drop for TermGuard {
        fn drop(&mut self) {
            let mut out = std::io::stdout();
            let _ = crossterm::execute!(out, crossterm::cursor::Show, crossterm::terminal::LeaveAlternateScreen);
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
        SetBackgroundColor(Color::Rgb { r: bg.r, g: bg.g, b: bg.b }),
        terminal::Clear(ClearType::All),
    )?;
    let _guard = TermGuard;

    let (cols, rows) = terminal::size()?;
    let mut scene = aa_core::scenes::make(scene_id).unwrap_or_else(|| {
        eprintln!("aa: unknown scene '{scene_id}', falling back to donut");
        aa_core::scenes::make("donut").unwrap()
    });
    scene.apply_base_color(theme.text);
    scene.set_grid(cols as usize, rows as usize);
    scene.start();

    let frame_dur = Duration::from_millis(1000 / fps.max(1) as u64);
    let start = Instant::now();

    loop {
        let ansi = aa_core::ansi::frame_to_ansi(&scene.frame(start.elapsed().as_secs_f64()), &theme);
        // frame_to_ansi already emits \x1b[2J\x1b[H, so write directly.
        print!("{ansi}");
        stdout.flush()?;

        // Poll for terminal events until the next frame deadline.
        let deadline = Instant::now() + frame_dur;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                break;
            }
            if event::poll(remaining)? {
                match event::read()? {
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
                                KeyCode::Up    => b"\x1b[A",
                                KeyCode::Down  => b"\x1b[B",
                                KeyCode::Right => b"\x1b[C",
                                KeyCode::Left  => b"\x1b[D",
                                KeyCode::Enter => b"\n",
                                KeyCode::Esc   => b"\x1b",
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
        return aa_linux::run(scene_id, RenderOptions { theme, ..Default::default() })
            .map_err(|e| e.to_string());
    }

    #[cfg(windows)]
    {
        use aa_render::RenderOptions;
        return aa_windows::run(scene_id, RenderOptions { theme, ..Default::default() })
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
            Err("ASCII Arcade.app not found — install it from the disk image first.\n\
                 For a terminal experience on any platform, try 'aa play'."
                .into())
        };
    }

    #[allow(unreachable_code)]
    Err("wallpaper mode is not supported on this platform; try 'aa play'".into())
}

// ── web ───────────────────────────────────────────────────────────────────────

async fn cmd_web(
    scene_id: String,
    theme: aa_core::Theme,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            Path, State,
        },
        response::{Html, IntoResponse, Json},
        routing::get,
        Router,
    };
    use std::time::{Duration, Instant};
    use tokio::time::MissedTickBehavior;

    #[derive(Clone, Copy)]
    struct AppState {
        theme: aa_core::Theme,
    }

    async fn root() -> Html<&'static str> {
        Html(include_str!("../../web/static/index.html"))
    }

    async fn scene_list() -> Json<&'static [&'static str]> {
        Json(aa_core::scenes::BUILTIN_IDS)
    }

    async fn ws_upgrade(
        Path(sid): Path<String>,
        ws: WebSocketUpgrade,
        State(s): State<AppState>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| run_ws(socket, sid, s.theme))
    }

    async fn run_ws(mut socket: WebSocket, sid: String, theme: aa_core::Theme) {
        let Some(mut scene) = aa_core::scenes::make(&sid) else {
            let _ = socket.send(Message::Text(format!("unknown scene: {sid}\r\n").into())).await;
            return;
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

    let _ = scene_id; // default scene lives in the URL; the CLI arg is informational
    let app = Router::new()
        .route("/", get(root))
        .route("/api/scenes", get(scene_list))
        .route("/ws/{scene_id}", get(ws_upgrade))
        .with_state(AppState { theme });

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
                println!(
                    "On macOS, toggle 'Launch at Login' in the ASCII Arcade status-bar menu."
                );
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
                println!(
                    "On macOS, toggle 'Launch at Login' in the ASCII Arcade status-bar menu."
                );
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
                println!(
                    "On macOS, check 'Launch at Login' in the ASCII Arcade status-bar menu."
                );
                return Ok(());
            }

            #[allow(unreachable_code)]
            Err("autostart status is not supported on this platform".into())
        }
    }
}
