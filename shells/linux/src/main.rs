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
        Some("--gnome-stream") => {
            let result = (|| {
                if args.len() < 5 {
                    return Err(
                        "usage: aa-linux --gnome-stream SCENE THEME WIDTH HEIGHT [OPTIONS]".into(),
                    );
                }
                let theme = Theme::by_name(&args[2]).ok_or("unknown theme")?;
                let width = args[3].parse::<u32>().map_err(|_| "invalid width")?;
                let height = args[4].parse::<u32>().map_err(|_| "invalid height")?;
                let mut opts = RenderOptions {
                    theme,
                    ..Default::default()
                };
                opts.opaque_background = !args.iter().any(|arg| arg == "--transparent");
                let settings = parse_settings(args);
                aa_linux::gnome::stream_with_controls(
                    aa_linux::gnome::GnomeStreamOptions {
                        scene_id: &args[1],
                        render: opts,
                        width,
                        height,
                        doom_enabled: args.iter().any(|arg| arg == "--enable-doom"),
                        settings: &settings,
                        capture_keys: args.iter().any(|arg| arg == "--capture-keys"),
                    },
                    &mut std::io::stdout().lock(),
                )
            })();
            return result.map_err(|e: String| {
                eprintln!("aa-linux: {e}");
                1
            });
        }
        Some("--autostart-enable") => {
            let scene = args.get(1).map(String::as_str).unwrap_or("donut");
            let theme = args.get(2).map(String::as_str).unwrap_or("hacker");
            let runtime_args = args
                .iter()
                .filter(|arg| {
                    matches!(
                        arg.as_str(),
                        "--enable-doom" | "--transparent" | "--idle-cycle" | "--capture-keys"
                    ) || arg.starts_with("--setting=")
                })
                .cloned()
                .collect::<Vec<_>>();
            if let Err(e) = aa_linux::autostart::install_with_args(scene, theme, &runtime_args) {
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
        Some("--autostart-status") => {
            println!("{}", aa_linux::autostart::is_installed());
            return Ok(());
        }
        _ => {}
    }

    let scene = args.first().map(String::as_str).unwrap_or("donut");
    let theme = args
        .get(1)
        .and_then(|n| Theme::by_name(n))
        .unwrap_or_default();

    let mut opts = RenderOptions {
        theme,
        ..Default::default()
    };
    opts.opaque_background = !args.iter().any(|arg| arg == "--transparent");

    if let Err(e) = aa_linux::run_with_controls(
        scene,
        opts,
        args.iter().any(|arg| arg == "--enable-doom"),
        &parse_settings(args),
        args.iter().any(|arg| arg == "--idle-cycle"),
        args.iter().any(|arg| arg == "--capture-keys"),
    ) {
        eprintln!("aa-linux: {e}");
        return Err(1);
    }
    Ok(())
}

fn parse_settings(args: &[String]) -> Vec<(String, f64)> {
    args.iter()
        .filter_map(|arg| arg.strip_prefix("--setting="))
        .filter_map(|setting| {
            let (id, value) = setting.split_once('=')?;
            let value = value.parse::<f64>().ok()?;
            value.is_finite().then(|| (id.to_owned(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnome_stream_rejects_invalid_arguments_without_opening_a_display() {
        for args in [
            vec!["--gnome-stream"],
            vec!["--gnome-stream", "donut", "missing", "16", "16"],
            vec!["--gnome-stream", "donut", "hacker", "NaN", "16"],
            vec!["--gnome-stream", "donut", "hacker", "2000", "16"],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert_eq!(run_cli(&args), Err(1));
        }
    }

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
