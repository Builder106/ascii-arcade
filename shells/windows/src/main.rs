//! `aa-windows` binary: parse a scene id + theme and launch the WorkerW host.
//!
//! Usage: `aa-windows [scene] [theme]`  (e.g. `aa-windows matrix amber`)
//!
//! Autostart management:
//!   `aa-windows --autostart-enable [scene] [theme]`  — install HKCU Run entry
//!   `aa-windows --autostart-disable`                 — remove HKCU Run entry

#![windows_subsystem = "windows"]

use aa_core::Theme;
use aa_render::RenderOptions;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(code) = run_cli(&args) {
        std::process::exit(code);
    }
}

fn run_cli(args: &[String]) -> Result<(), i32> {
    match args.first().map(String::as_str) {
        Some("--autostart-enable") => {
            let scene = args.get(1).map(String::as_str).unwrap_or("donut");
            let theme = args.get(2).map(String::as_str).unwrap_or("hacker");
            if let Err(e) = aa_windows::autostart::install(scene, theme) {
                eprintln!("aa-windows: {e}");
                return Err(1);
            }
            return Ok(());
        }
        Some("--autostart-disable") => {
            if let Err(e) = aa_windows::autostart::remove() {
                eprintln!("aa-windows: {e}");
                return Err(1);
            }
            return Ok(());
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

    if let Err(e) = aa_windows::run(scene, opts) {
        eprintln!("aa-windows: {e}");
        return Err(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cli_autostart_flags() {
        let enable_args = vec![
            "--autostart-enable".to_string(),
            "donut".to_string(),
            "hacker".to_string(),
        ];
        #[cfg(not(windows))]
        {
            assert_eq!(run_cli(&enable_args), Err(1));

            let disable_args = vec!["--autostart-disable".to_string()];
            assert_eq!(run_cli(&disable_args), Err(1));
        }

        #[cfg(windows)]
        {
            let _ = run_cli(&enable_args);
        }
    }

    #[test]
    fn run_cli_default_args_error_handling() {
        #[cfg(not(windows))]
        {
            let args = vec!["donut".to_string(), "amber".to_string()];
            assert_eq!(run_cli(&args), Err(1));

            let empty_args = vec![];
            assert_eq!(run_cli(&empty_args), Err(1));
        }

        #[cfg(windows)]
        {
            let args = vec!["invalid_scene_xyz".to_string()];
            assert_eq!(run_cli(&args), Err(1));
        }
    }
}
