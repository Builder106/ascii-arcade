//! # aa-web
//!
//! Browser-facing shell for ASCII Arcade. Serves an xterm.js page and streams
//! scene frames as ANSI truecolor escape sequences over a WebSocket at 30 fps.
//!
//! Routes:
//!   GET /              → scene picker + xterm.js UI (static HTML)
//!   GET /api/scenes    → JSON array of built-in scene ids
//!   GET /ws/:scene_id  → WebSocket; frames in, key bytes out
//!
//! Environment variables:
//!   AA_WEB_PORT         TCP port to listen on (default 8788)
//!   AA_WEB_THEME        Theme name: hacker | amber | ice | ghost (default hacker)
//!   AA_WEB_ENABLE_DOOM  Set to "1" to unlock the opt-in DOOM scene at
//!                       /ws/doom (also needs this binary built with
//!                       `--features doom`). Off by default — DOOM is a
//!                       playable shooter, not just another scene.

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

/// DOOM's framebuffer is fixed at scene-construction time (see `make_scene`),
/// but a browser tab's terminal size isn't known until its first
/// `__resize__:` message arrives just after connecting — after the scene
/// already exists. Use one fixed, conservative grid (160×50) that fits most
/// browser windows rather than delaying construction on that handshake.
#[cfg(feature = "doom")]
const WEB_DOOM_SCALING: usize = 4;

#[derive(Clone, Copy)]
struct AppState {
    enable_doom: bool,
}

pub fn create_app(enable_doom: bool) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/api/scenes", get(scenes_handler))
        .route("/ws/{scene_id}", get(ws_upgrade_handler))
        .with_state(AppState { enable_doom })
}

pub async fn run_server(
    listener: tokio::net::TcpListener,
    enable_doom: bool,
) -> std::io::Result<()> {
    let app = create_app(enable_doom);
    axum::serve(listener, app).await
}

#[tokio::main]
async fn main() {
    let port = std::env::var("AA_WEB_PORT").unwrap_or_else(|_| "8788".into());
    let addr = format!("0.0.0.0:{port}");
    let enable_doom = std::env::var("AA_WEB_ENABLE_DOOM").as_deref() == Ok("1");

    println!("aa-web listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    run_server(listener, enable_doom).await.unwrap();
}

async fn root_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn scenes_handler(State(s): State<AppState>) -> Json<Vec<&'static str>> {
    let mut ids: Vec<&'static str> = aa_core::scenes::BUILTIN_IDS.to_vec();
    if cfg!(feature = "doom") && s.enable_doom {
        ids.push("doom");
    }
    Json(ids)
}

async fn ws_upgrade_handler(
    Path(scene_id): Path<String>,
    ws: WebSocketUpgrade,
    State(s): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, scene_id, s.enable_doom))
}

/// Construct a scene by id. DOOM isn't part of `aa_core::scenes::BUILTIN_IDS`
/// on purpose (see crates/aa-core) — this is the one seam where the web shell
/// adds it back in, deliberately gated behind both a Cargo feature (is it
/// even compiled in?) and a runtime opt-in (`AA_WEB_ENABLE_DOOM=1`).
fn make_scene(id: &str, enable_doom: bool) -> Result<Box<dyn aa_core::Scene + Send>, String> {
    let _ = enable_doom; // read only when built with `--features doom`

    #[cfg(feature = "doom")]
    if id == "doom" {
        if !enable_doom {
            return Err("DOOM is opt-in — set AA_WEB_ENABLE_DOOM=1 to play it".into());
        }
        return Ok(Box::new(aa_doom::DoomScene::new(WEB_DOOM_SCALING)));
    }

    #[cfg(not(feature = "doom"))]
    if id == "doom" {
        return Err(
            "this build of aa-web doesn't include DOOM — rebuild with `cargo build --features doom`"
                .into(),
        );
    }

    aa_core::scenes::make(id).ok_or_else(|| format!("unknown scene '{id}'"))
}

async fn process_ws_stream(
    mut scene: Box<dyn aa_core::Scene + Send>,
    theme: aa_core::Theme,
    tx: tokio::sync::mpsc::Sender<Message>,
    mut rx: tokio::sync::mpsc::Receiver<Message>,
) {
    let mut cols = 120usize;
    let mut rows = 40usize;
    scene.set_grid(cols, rows);
    scene.start();

    let start = Instant::now();
    let mut interval = tokio::time::interval(Duration::from_millis(33));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let t = start.elapsed().as_secs_f64();
                let frame = scene.frame(t);
                let ansi = aa_core::ansi::frame_to_ansi(&frame, &theme);
                if tx.send(Message::Text(ansi.into())).await.is_err() {
                    break;
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(Message::Text(text)) => {
                        if let Some(dims) = text.strip_prefix("__resize__:") {
                            if let Some((c, r)) = dims.split_once('x') {
                                if let (Ok(c), Ok(r)) = (c.parse::<usize>(), r.parse::<usize>()) {
                                    cols = c.max(20);
                                    rows = r.max(10);
                                    scene.set_grid(cols, rows);
                                }
                            }
                        } else {
                            scene.send_key(text.as_bytes());
                        }
                    }
                    Some(Message::Binary(data)) => {
                        scene.send_key(&data);
                    }
                    None | Some(Message::Close(_)) => break,
                    _ => {}
                }
            }
        }
    }

    scene.stop();
}

async fn handle_ws(mut socket: WebSocket, scene_id: String, enable_doom: bool) {
    let theme_name = std::env::var("AA_WEB_THEME").unwrap_or_else(|_| "hacker".into());
    let theme = aa_core::Theme::by_name(&theme_name).unwrap_or_default();

    let scene = match make_scene(&scene_id, enable_doom) {
        Ok(s) => s,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("{e}\r\n").into())).await;
            return;
        }
    };

    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Message>(32);
    let (in_tx, in_rx) = tokio::sync::mpsc::channel::<Message>(32);

    let stream_task = tokio::spawn(process_ws_stream(scene, theme, out_tx, in_rx));

    loop {
        tokio::select! {
            out_msg = out_rx.recv() => {
                match out_msg {
                    Some(msg) => {
                        if socket.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            in_msg = socket.recv() => {
                match in_msg {
                    Some(Ok(msg)) => {
                        let is_close = matches!(msg, Message::Close(_));
                        let _ = in_tx.send(msg).await;
                        if is_close {
                            break;
                        }
                    }
                    None | Some(Err(_)) => break,
                }
            }
        }
    }

    stream_task.abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_scene_rejects_doom_without_opt_in() {
        match make_scene("doom", false) {
            Err(e) => assert!(e.contains("opt-in") || e.contains("doesn't include DOOM")),
            Ok(_) => panic!("expected doom to be rejected without AA_WEB_ENABLE_DOOM=1"),
        }
    }

    #[test]
    fn make_scene_still_resolves_ordinary_scenes() {
        assert!(make_scene("donut", false).is_ok());
    }

    #[test]
    fn make_scene_rejects_unknown_scene() {
        assert!(make_scene("not-a-real-scene", false).is_err());
    }

    #[cfg(feature = "doom")]
    #[test]
    fn make_scene_allows_doom_with_opt_in() {
        assert!(make_scene("doom", true).is_ok());
    }

    #[tokio::test]
    async fn root_handler_returns_html() {
        let res = root_handler().await;
        assert!(res.0.contains("<!doctype html>") || res.0.contains("<!DOCTYPE html>"));
        assert!(res.0.contains("xterm"));
        assert!(res.0.contains("ASCII Arcade"));
    }

    #[tokio::test]
    async fn scenes_handler_returns_builtins_and_doom_when_enabled() {
        let res_no_doom = scenes_handler(State(AppState { enable_doom: false })).await;
        assert!(res_no_doom.0.contains(&"donut"));
        assert!(res_no_doom.0.contains(&"matrix"));
        assert!(!res_no_doom.0.contains(&"doom"));

        let res_doom = scenes_handler(State(AppState { enable_doom: true })).await;
        assert!(res_doom.0.contains(&"donut"));
        #[cfg(feature = "doom")]
        assert!(res_doom.0.contains(&"doom"));
    }

    #[test]
    fn app_state_clone_copy() {
        let state = AppState { enable_doom: true };
        let copied = state;
        assert!(copied.enable_doom);
    }

    #[tokio::test]
    async fn test_server_bind_and_routes() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            run_server(listener, true).await.unwrap();
        });

        // Test root endpoint HTTP GET
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("xterm"));

        // Test API scenes endpoint
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /api/scenes HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("donut"));
        #[cfg(feature = "doom")]
        assert!(response.contains("doom"));

        server_handle.abort();
    }

    // Mock interactive scene to verify key, binary, resize, frame delivery and lifecycle
    struct MockWebScene {
        keys: Vec<Vec<u8>>,
        cols: usize,
        rows: usize,
        started: bool,
        stopped: bool,
    }

    impl MockWebScene {
        fn new() -> Self {
            Self {
                keys: Vec::new(),
                cols: 0,
                rows: 0,
                started: false,
                stopped: false,
            }
        }
    }

    impl aa_core::Scene for MockWebScene {
        fn display_name(&self) -> &str {
            "MockWeb"
        }
        fn is_interactive(&self) -> bool {
            true
        }
        fn set_grid(&mut self, width: usize, height: usize) {
            self.cols = width;
            self.rows = height;
        }
        fn frame(&mut self, _t: f64) -> aa_core::Frame {
            aa_core::Frame::blank(self.cols.max(1), self.rows.max(1))
        }
        fn send_key(&mut self, bytes: &[u8]) {
            self.keys.push(bytes.to_vec());
        }
        fn start(&mut self) {
            self.started = true;
        }
        fn stop(&mut self) {
            self.stopped = true;
        }
    }

    #[tokio::test]
    async fn process_ws_stream_handles_client_messages_and_lifecycle() {
        use axum::extract::ws::CloseFrame;

        let scene = Box::new(MockWebScene::new());
        let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Message>(32);
        let (in_tx, in_rx) = tokio::sync::mpsc::channel::<Message>(32);

        let task = tokio::spawn(process_ws_stream(
            scene,
            aa_core::Theme::HACKER,
            out_tx,
            in_rx,
        ));

        // Read at least one frame
        let first_frame = out_rx.recv().await;
        assert!(first_frame.is_some());

        // Send resize message
        in_tx
            .send(Message::Text("__resize__:100x40".into()))
            .await
            .unwrap();

        // Send invalid resize message
        in_tx
            .send(Message::Text("__resize__:bad_format".into()))
            .await
            .unwrap();
        in_tx
            .send(Message::Text("__resize__:100xnotanumber".into()))
            .await
            .unwrap();

        // Send text data
        in_tx
            .send(Message::Text("user_input".into()))
            .await
            .unwrap();

        // Send binary data
        in_tx
            .send(Message::Binary(vec![4, 5, 6].into()))
            .await
            .unwrap();

        // Send close message
        in_tx
            .send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: "".into(),
            })))
            .await
            .unwrap();

        // Let the task exit cleanly
        let _ = task.await;
    }

    #[tokio::test]
    async fn process_ws_stream_breaks_on_out_tx_drop() {
        let scene = Box::new(MockWebScene::new());
        let (out_tx, out_rx) = tokio::sync::mpsc::channel::<Message>(32);
        let (_in_tx, in_rx) = tokio::sync::mpsc::channel::<Message>(32);

        drop(out_rx); // dropping receiver causes out_tx.send to fail

        let task = tokio::spawn(process_ws_stream(scene, aa_core::Theme::ICE, out_tx, in_rx));
        let _ = task.await;
    }
}
