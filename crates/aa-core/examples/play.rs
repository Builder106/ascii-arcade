//! Headless terminal renderer — the parity / eyeball harness.
//!
//! Renders a built-in scene to the terminal using ANSI cursor-home so frames
//! animate in place. Used to compare the Rust port against the Swift output and
//! to sanity-check a scene without a wallpaper shell.
//!
//! Usage:
//!   cargo run -p aa-core --example play -- [scene] [cols] [rows] [seconds]
//!   cargo run -p aa-core --example play -- donut 80 40 5

use aa_core::scenes;
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id = args.first().map(String::as_str).unwrap_or("donut");
    let cols: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(80);
    let rows: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    let seconds: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(5.0);

    let Some(mut scene) = scenes::make(id) else {
        eprintln!("unknown scene '{id}'. known: {:?}", scenes::BUILTIN_IDS);
        std::process::exit(1);
    };
    scene.set_grid(cols, rows);

    let mut out = std::io::stdout().lock();
    let start = Instant::now();
    print!("\x1b[2J"); // clear once
    while start.elapsed().as_secs_f64() < seconds {
        let t = start.elapsed().as_secs_f64();
        let frame = scene.frame(t);
        let _ = write!(out, "\x1b[H{}", frame.text()); // cursor home, no full clear
        let _ = out.flush();
        sleep(Duration::from_millis(33));
    }
}
