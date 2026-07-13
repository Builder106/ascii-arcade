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

#[tokio::main]
async fn main() {
    let port = std::env::var("AA_WEB_PORT").unwrap_or_else(|_| "8788".into());
    let addr = format!("0.0.0.0:{port}");
    let enable_doom = std::env::var("AA_WEB_ENABLE_DOOM").as_deref() == Ok("1");

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/scenes", get(scenes_handler))
        .route("/ws/{scene_id}", get(ws_upgrade_handler))
        .with_state(AppState { enable_doom });

    println!("aa-web listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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

async fn handle_ws(mut socket: WebSocket, scene_id: String, enable_doom: bool) {
    let theme_name = std::env::var("AA_WEB_THEME").unwrap_or_else(|_| "hacker".into());
    let theme = aa_core::Theme::by_name(&theme_name).unwrap_or_default();

    let mut scene = match make_scene(&scene_id, enable_doom) {
        Ok(s) => s,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("{e}\r\n").into())).await;
            return;
        }
    };

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
                if socket.send(Message::Text(ansi.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
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
                    Some(Ok(Message::Binary(data))) => {
                        scene.send_key(&data);
                    }
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
        }
    }

    scene.stop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_scene_rejects_doom_without_opt_in() {
        // Not `.unwrap_err()`: that needs `Box<dyn Scene + Send>: Debug`,
        // which trait objects over `Scene` don't implement.
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
}
