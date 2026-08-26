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
    if let Err(code) = run_cli(&args) {
        std::process::exit(code);
    }
}

fn run_cli(args: &[String]) -> Result<(), i32> {
    match args.first().map(String::as_str) {
        Some("--autostart-enable") => {
            let scene = args.get(1).map(String::as_str).unwrap_or("donut");
            let theme = args.get(2).map(String::as_str).unwrap_or("hacker");
            if let Err(e) = aa_linux::autostart::install(scene, theme) {
                eprintln!("aa-linux: {e}");
                return Err(1);
            }
            return Ok(());
        }
        Some("--autostart-disable") => {
            if let Err(e) = aa_linux::autostart::remove() {
                eprintln!("aa-linux: {e}");
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

    if let Err(e) = aa_linux::run(scene, opts) {
        eprintln!("aa-linux: {e}");
        return Err(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cli_autostart_enable_and_disable() {
        let temp_home =
            std::env::temp_dir().join(format!("aa-linux-cli-autostart-{}", std::process::id()));
        std::fs::create_dir_all(&temp_home).unwrap();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &temp_home);

        #[cfg(target_os = "linux")]
        {
            let enable_args = vec![
                "--autostart-enable".to_string(),
                "matrix".to_string(),
                "amber".to_string(),
            ];
            assert_eq!(run_cli(&enable_args), Ok(()));
            assert!(aa_linux::autostart::is_installed());

            let disable_args = vec!["--autostart-disable".to_string()];
            assert_eq!(run_cli(&disable_args), Ok(()));
            assert!(!aa_linux::autostart::is_installed());
        }

        #[cfg(not(target_os = "linux"))]
        {
            let enable_args = vec!["--autostart-enable".to_string()];
            assert_eq!(run_cli(&enable_args), Err(1));

            let disable_args = vec!["--autostart-disable".to_string()];
            assert_eq!(run_cli(&disable_args), Err(1));
        }

        if let Some(h) = old_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
        std::fs::remove_dir_all(&temp_home).ok();
    }

    #[test]
    fn run_cli_default_args_error_handling() {
        // Without active X11/Wayland display server or on invalid scene, run_cli returns Err(1)
        let args = vec![
            "unknown_scene_123".to_string(),
            "unknown_theme_456".to_string(),
        ];
        assert_eq!(run_cli(&args), Err(1));

        let empty_args = vec![];
        assert_eq!(run_cli(&empty_args), Err(1));
    }
}
