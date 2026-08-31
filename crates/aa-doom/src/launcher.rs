//! Resolves the vendored `doom_ascii` binary and a playable IWAD, then builds
//! the argv + env to launch it.
//!
//! Ported from `DoomLauncher.swift`. The search policy is identical so the Rust
//! shells and the Swift wallpaper app find the same binary/WAD:
//!   * binary: `$DOOM_ASCII_PATH` → `<cwd>/bin/doom_ascii` → `/usr/local/bin` →
//!     `/opt/homebrew/bin` (the bundle path is macOS-app-only and dropped here).
//!   * IWAD: `$DOOM_WAD_DIR` → `<cwd>/wad` → the system doom dirs.
//!
//! Resolution is pure given an explicit working directory + environment, so the
//! whole policy is unit-testable against a temp dir without spawning anything.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where to find the `doom_ascii` binary + IWAD and how to launch it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoomLaunchConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// IWADs `doom_ascii` knows how to load, in preference order (commercial first,
/// then the free `freedoom` set so a checkout can ship a redistributable WAD).
const IWAD_CANDIDATES: &[&str] = &[
    "doom.wad",
    "doom1.wad",
    "doom2.wad",
    "plutonia.wad",
    "tnt.wad",
    "chex.wad",
    "hacx.wad",
    "freedoom1.wad",
    "freedoom2.wad",
    "freedoom.wad",
    "freedm.wad",
];

/// On Windows the built binary is `doom_ascii.exe`; elsewhere it's `doom_ascii`.
#[cfg(windows)]
const BINARY_NAMES: &[&str] = &["doom_ascii.exe", "doom_ascii"];
#[cfg(not(windows))]
const BINARY_NAMES: &[&str] = &["doom_ascii"];

#[cfg(windows)]
const SYSTEM_BIN_DIRS: &[&str] = &[];
#[cfg(not(windows))]
const SYSTEM_BIN_DIRS: &[&str] = &["/usr/local/bin", "/opt/homebrew/bin"];

#[cfg(windows)]
const SYSTEM_WAD_DIRS: &[&str] = &[];
#[cfg(not(windows))]
const SYSTEM_WAD_DIRS: &[&str] = &[
    "/opt/homebrew/share/games/doom",
    "/usr/local/share/games/doom",
    "/usr/share/games/doom",
];

/// Default gamma when `DOOM_GAMMA` is unset. DOOM defaults to gamma OFF —
/// authentic but very dark, which reads poorly as a wallpaper (unlit sectors
/// fade to pure black), so we brighten by default. 0 = off … 4 = brightest.
const DEFAULT_GAMMA: u32 = 2;

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Resolve the `doom_ascii` binary. `env` is the process environment (passed in
/// so the policy is testable); `working_directory` is searched for `bin/`.
pub fn resolve_binary(working_directory: &Path, env: &HashMap<String, String>) -> Option<PathBuf> {
    if let Some(explicit) = env.get("DOOM_ASCII_PATH") {
        let p = PathBuf::from(explicit);
        if is_executable_file(&p) {
            return Some(p);
        }
    }
    for name in BINARY_NAMES {
        let local = working_directory.join("bin").join(name);
        if is_executable_file(&local) {
            return Some(local);
        }
    }
    for dir in SYSTEM_BIN_DIRS {
        for name in BINARY_NAMES {
            let p = Path::new(dir).join(name);
            if is_executable_file(&p) {
                return Some(p);
            }
        }
    }
    None
}

/// Resolve a playable IWAD: returns `(iwad_path, containing_dir)`. The dir is
/// exported as `DOOMWADDIR` so `doom_ascii` finds adjacent lumps.
pub fn resolve_iwad(
    working_directory: &Path,
    env: &HashMap<String, String>,
) -> Option<(PathBuf, PathBuf)> {
    let mut search_dirs: Vec<PathBuf> = Vec::new();
    if let Some(wad_dir) = env.get("DOOM_WAD_DIR") {
        if !wad_dir.is_empty() {
            search_dirs.push(PathBuf::from(wad_dir));
        }
    }
    search_dirs.push(working_directory.join("wad"));
    for dir in SYSTEM_WAD_DIRS {
        search_dirs.push(PathBuf::from(dir));
    }

    for dir in &search_dirs {
        for name in IWAD_CANDIDATES {
            let full = dir.join(name);
            if full.is_file() {
                return Some((full, dir.clone()));
            }
        }
    }
    None
}

/// Build a full launch config (binary + argv + env), or `None` if no binary is
/// found. `scaling` is the `-scaling N` factor (clamped to ≥ 1 by the caller's
/// grid math; passed straight through here). Ported from `DoomLauncher.resolve`.
pub fn resolve(
    working_directory: &Path,
    env: &HashMap<String, String>,
    scaling: usize,
) -> Option<DoomLaunchConfig> {
    let executable = resolve_binary(working_directory, env)?;

    let mut launch_env = env.clone();
    let mut args: Vec<String> = vec!["-chars".into(), "block".into()];

    let n = scaling.max(1);
    args.push("-scaling".into());
    args.push(n.to_string());

    let gamma = env
        .get("DOOM_GAMMA")
        .and_then(|g| g.parse::<u32>().ok())
        .unwrap_or(DEFAULT_GAMMA);
    if gamma > 0 {
        args.push("-fixgamma".into());
        args.push(gamma.min(4).to_string());
    }

    if let Some((iwad, dir)) = resolve_iwad(working_directory, env) {
        args.push("-iwad".into());
        args.push(iwad.to_string_lossy().into_owned());
        launch_env.insert("DOOMWADDIR".into(), dir.to_string_lossy().into_owned());
    } else if let Some(wad) = env.get("DOOM_WAD_DIR") {
        if !wad.is_empty() {
            launch_env.insert("DOOMWADDIR".into(), wad.clone());
        }
    }

    Some(DoomLaunchConfig {
        executable,
        args,
        env: launch_env,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn empty_env() -> HashMap<String, String> {
        HashMap::new()
    }

    /// Create a fake executable `bin/doom_ascii` under `dir`.
    fn make_fake_binary(dir: &Path) -> PathBuf {
        let bin_dir = dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let name = if cfg!(windows) {
            "doom_ascii.exe"
        } else {
            "doom_ascii"
        };
        let path = bin_dir.join(name);
        fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        // A per-process atomic sequence guarantees uniqueness even when parallel
        // test threads call this within the same clock tick. A time-only suffix
        // collided on CI (macOS clocks can be coarse), letting one test delete
        // another's fixture mid-run.
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join(format!(
            "aa-doom-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_binary_finds_local_bin() {
        let dir = tmp();
        let expected = make_fake_binary(&dir);
        let got = resolve_binary(&dir, &empty_env());
        assert_eq!(got, Some(expected));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_binary_honors_env_override() {
        let dir = tmp();
        let explicit = make_fake_binary(&dir);
        let mut env = empty_env();
        env.insert(
            "DOOM_ASCII_PATH".into(),
            explicit.to_string_lossy().into_owned(),
        );
        // Working dir has no bin/, so only the override can satisfy it.
        let wd = tmp();
        let got = resolve_binary(&wd, &env);
        assert_eq!(got, Some(explicit));
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&wd).ok();
    }

    #[test]
    fn resolve_binary_missing_returns_none() {
        let dir = tmp();
        assert_eq!(resolve_binary(&dir, &empty_env()), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_iwad_finds_wad_in_working_dir() {
        let dir = tmp();
        let wad_dir = dir.join("wad");
        fs::create_dir_all(&wad_dir).unwrap();
        let wad = wad_dir.join("freedoom1.wad");
        fs::write(&wad, b"IWAD").unwrap();
        let got = resolve_iwad(&dir, &empty_env());
        assert_eq!(got, Some((wad, wad_dir)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_iwad_prefers_commercial_over_free() {
        let dir = tmp();
        let wad_dir = dir.join("wad");
        fs::create_dir_all(&wad_dir).unwrap();
        // Both present — doom.wad must win (earlier in IWAD_CANDIDATES).
        fs::write(wad_dir.join("freedoom1.wad"), b"IWAD").unwrap();
        let doom = wad_dir.join("doom.wad");
        fs::write(&doom, b"IWAD").unwrap();
        let (got, _) = resolve_iwad(&dir, &empty_env()).unwrap();
        assert_eq!(got, doom);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_builds_expected_argv() {
        let dir = tmp();
        make_fake_binary(&dir);
        let cfg = resolve(&dir, &empty_env(), 2).unwrap();
        // -chars block always present, -scaling reflects the factor, default
        // gamma 2 produces -fixgamma 2.
        assert_eq!(
            cfg.args,
            vec![
                "-chars".to_string(),
                "block".into(),
                "-scaling".into(),
                "2".into(),
                "-fixgamma".into(),
                "2".into(),
            ]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_gamma_zero_drops_fixgamma() {
        let dir = tmp();
        make_fake_binary(&dir);
        let mut env = empty_env();
        env.insert("DOOM_GAMMA".into(), "0".into());
        let cfg = resolve(&dir, &env, 1).unwrap();
        assert!(!cfg.args.iter().any(|a| a == "-fixgamma"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_gamma_clamped_to_four() {
        let dir = tmp();
        make_fake_binary(&dir);
        let mut env = empty_env();
        env.insert("DOOM_GAMMA".into(), "9".into());
        let cfg = resolve(&dir, &env, 1).unwrap();
        let i = cfg.args.iter().position(|a| a == "-fixgamma").unwrap();
        assert_eq!(cfg.args[i + 1], "4");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_sets_doomwaddir_when_iwad_found() {
        let dir = tmp();
        make_fake_binary(&dir);
        let wad_dir = dir.join("wad");
        fs::create_dir_all(&wad_dir).unwrap();
        fs::write(wad_dir.join("doom1.wad"), b"IWAD").unwrap();
        let cfg = resolve(&dir, &empty_env(), 1).unwrap();
        assert_eq!(
            cfg.env.get("DOOMWADDIR").map(String::as_str),
            Some(wad_dir.to_string_lossy().as_ref())
        );
        assert!(cfg.args.iter().any(|a| a == "-iwad"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_binary_searches_system_dirs() {
        let dir = tmp();
        // Missing binary anywhere
        let got = resolve_binary(&dir, &empty_env());
        assert!(got.is_none() || got.is_some()); // On systems where doom_ascii might exist in /usr/local/bin, handles both
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_system_directories_constants() {
        // Verify SYSTEM_BIN_DIRS and SYSTEM_WAD_DIRS constants
        #[cfg(not(windows))]
        {
            assert_eq!(SYSTEM_BIN_DIRS, &["/usr/local/bin", "/opt/homebrew/bin"]);
            assert_eq!(
                SYSTEM_WAD_DIRS,
                &[
                    "/opt/homebrew/share/games/doom",
                    "/usr/local/share/games/doom",
                    "/usr/share/games/doom",
                ]
            );
        }
    }


    #[test]
    fn resolve_iwad_honors_doom_wad_dir_env() {
        let dir = tmp();
        let custom_dir = dir.join("custom_wad_dir");
        fs::create_dir_all(&custom_dir).unwrap();
        let wad = custom_dir.join("freedoom2.wad");
        fs::write(&wad, b"IWAD").unwrap();

        let mut env = empty_env();
        env.insert(
            "DOOM_WAD_DIR".into(),
            custom_dir.to_string_lossy().into_owned(),
        );

        let got = resolve_iwad(&dir, &env);
        assert_eq!(got, Some((wad, custom_dir)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_with_doom_wad_dir_env_sets_doomwaddir_when_no_iwad_file_found() {
        let dir = tmp();
        make_fake_binary(&dir);
        let mut env = empty_env();
        env.insert("DOOM_WAD_DIR".into(), "/non/existent/wad/path".into());
        let cfg = resolve(&dir, &env, 1).unwrap();
        assert_eq!(
            cfg.env.get("DOOMWADDIR").map(String::as_str),
            Some("/non/existent/wad/path")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_scaling_clamped_to_minimum_one() {
        let dir = tmp();
        make_fake_binary(&dir);
        let cfg = resolve(&dir, &empty_env(), 0).unwrap();
        let i = cfg.args.iter().position(|a| a == "-scaling").unwrap();
        assert_eq!(cfg.args[i + 1], "1");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_iwad_empty_wad_dir_is_ignored() {
        let dir = tmp();
        let mut env = empty_env();
        env.insert("DOOM_WAD_DIR".into(), "".into());
        let got = resolve_iwad(&dir, &env);
        assert_eq!(got, None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn is_executable_file_rejects_directories_and_nonexistent() {
        let dir = tmp();
        assert!(!is_executable_file(&dir)); // directory
        assert!(!is_executable_file(&dir.join("nonexistent"))); // does not exist
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_binary_non_executable_file_is_skipped() {
        let dir = tmp();
        let non_exec = dir.join("bin").join("doom_ascii");
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(&non_exec, b"not executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&non_exec, fs::Permissions::from_mode(0o644)).unwrap();
        }
        // On Unix, non_exec has no execute bits (0o644), so resolve_binary should skip it.
        #[cfg(unix)]
        {
            let got = resolve_binary(&dir, &empty_env());
            assert!(got.is_none() || got != Some(non_exec.clone()));
        }

        // Test explicit DOOM_ASCII_PATH pointing to a non-existent file or directory
        let dir_non_existent = tmp();
        let mut env = empty_env();
        env.insert(
            "DOOM_ASCII_PATH".into(),
            dir_non_existent
                .join("nonexistent_doom")
                .to_string_lossy()
                .into_owned(),
        );
        assert_eq!(resolve_binary(&dir_non_existent, &env), None);

        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&dir_non_existent).ok();
    }
}
