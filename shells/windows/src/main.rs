//! `aa-windows` binary: parse a scene id + theme and launch the WorkerW host.
//!
//! Usage: `aa-windows [scene] [theme]`  (e.g. `aa-windows matrix amber`)

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

    if let Err(e) = aa_windows::run(scene, opts) {
        eprintln!("aa-windows: {e}");
        std::process::exit(1);
    }
}
