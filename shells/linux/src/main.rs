//! `aa-linux` binary: parse a scene id + theme and launch the wallpaper host on
//! the detected backend (X11 or Wayland).
//!
//! Usage: `aa-linux [scene] [theme]`  (e.g. `aa-linux pipes ice`)
//!
//! Autostart management:
//!   `aa-linux --autostart-enable [scene] [theme]`  — install XDG desktop entry
//!   `aa-linux --autostart-disable`                 — remove XDG desktop entry

use aa_core::Theme;
use aa_render::RenderOptions;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("--autostart-enable") => {
            let scene = args.get(1).map(String::as_str).unwrap_or("donut");
            let theme = args.get(2).map(String::as_str).unwrap_or("hacker");
            if let Err(e) = aa_linux::autostart::install(scene, theme) {
                eprintln!("aa-linux: {e}");
                std::process::exit(1);
            }
            return;
        }
        Some("--autostart-disable") => {
            if let Err(e) = aa_linux::autostart::remove() {
                eprintln!("aa-linux: {e}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    let scene = args.first().map(String::as_str).unwrap_or("donut");
    let theme = args
        .get(1)
        .and_then(|n| Theme::by_name(n))
        .unwrap_or_default();

    let opts = RenderOptions {
        theme,
        ..Default::default()
    };

    if let Err(e) = aa_linux::run(scene, opts) {
        eprintln!("aa-linux: {e}");
        std::process::exit(1);
    }
}
