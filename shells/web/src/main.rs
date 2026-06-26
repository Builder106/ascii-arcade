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
//!   AA_WEB_PORT   TCP port to listen on (default 8788)
//!   AA_WEB_THEME  Theme name: hacker | amber | ice | ghost (default hacker)

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use std::time::{Duration, Instant};
use tokio::time::MissedTickBehavior;

#[tokio::main]
async fn main() {
    let port = std::env::var("AA_WEB_PORT").unwrap_or_else(|_| "8788".into());
    let addr = format!("0.0.0.0:{port}");

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/scenes", get(scenes_handler))
        .route("/ws/{scene_id}", get(ws_upgrade_handler));

    println!("aa-web listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn scenes_handler() -> Json<&'static [&'static str]> {
    Json(aa_core::scenes::BUILTIN_IDS)
}

async fn ws_upgrade_handler(
    Path(scene_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, scene_id))
}

async fn handle_ws(mut socket: WebSocket, scene_id: String) {
    let theme_name = std::env::var("AA_WEB_THEME").unwrap_or_else(|_| "hacker".into());
    let theme = aa_core::Theme::by_name(&theme_name).unwrap_or_default();

    let Some(mut scene) = aa_core::scenes::make(&scene_id) else {
        let _ = socket
            .send(Message::Text(
                format!("Unknown scene: {scene_id}\r\n").into(),
            ))
            .await;
        return;
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
