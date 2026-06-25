//! `aa-linux` binary: parse a scene id + theme and launch the wallpaper host on
//! the detected backend (X11 or Wayland).
//!
//! Usage: `aa-linux [scene] [theme]`  (e.g. `aa-linux pipes ice`)

use aa_core::Theme;
use aa_render::RenderOptions;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
