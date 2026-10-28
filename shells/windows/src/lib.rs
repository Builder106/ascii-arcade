//! # aa-windows
//!
//! The Windows wallpaper shell. Uses the **WorkerW** technique: send the
//! undocumented `0x052C` message to `Progman`, which spawns a `WorkerW` window
//! *behind* the desktop icons; enumerate top-level windows to find it, then
//! `SetParent` our render window into it. Each frame we rasterise the active
//! scene with [`aa_render`] and blit the RGBA buffer with GDI `StretchDIBits`.
//! (Lively Wallpaper is the open-source C# reference for the same trick.)
//!
//! The real implementation is `#[cfg(windows)]` and built on CI
//! (`windows-latest`); off-Windows this crate is a stub returning
//! [`ShellError::Unsupported`] so the workspace still checks on macOS/Linux.

use aa_render::RenderOptions;

/// Errors the shell can surface to its caller.
#[derive(Debug)]
pub enum ShellError {
    /// Built for a non-Windows target.
    Unsupported,
    /// A Win32 call failed.
    #[cfg(windows)]
    Win32(String),
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::Unsupported => write!(f, "the Windows shell only runs on Windows"),
            #[cfg(windows)]
            ShellError::Win32(s) => write!(f, "win32 error: {s}"),
        }
    }
}

impl std::error::Error for ShellError {}

/// Launch the wallpaper host: own the WorkerW surface and run the render loop
/// until the desktop session ends. `scene_id` is an `aa_core::scenes` id;
/// `opts` controls the rasteriser (theme, cell size, CRT FX).
pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
    run_with_doom(scene_id, opts, false)
}

/// Launch the Windows wallpaper host, exposing DOOM only when the user opted in.
pub fn run_with_doom(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
) -> Result<(), ShellError> {
    run_with_doom_settings(scene_id, opts, doom_enabled, &[])
}

pub fn run_with_doom_settings(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
) -> Result<(), ShellError> {
    run_with_controls(scene_id, opts, doom_enabled, settings, false, false)
}

pub fn run_with_controls(
    scene_id: &str,
    opts: RenderOptions,
    doom_enabled: bool,
    settings: &[(String, f64)],
    idle_cycle: bool,
    capture_keys: bool,
) -> Result<(), ShellError> {
    #[cfg(windows)]
    {
        imp::run(
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (
            scene_id,
            opts,
            doom_enabled,
            settings,
            idle_cycle,
            capture_keys,
        );
        Err(ShellError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_error_display_and_error_impl() {
        let unsupp = ShellError::Unsupported;
        assert_eq!(
            format!("{unsupp}"),
            "the Windows shell only runs on Windows"
        );
        assert!((&unsupp as &dyn std::error::Error).source().is_none());

        #[cfg(windows)]
        {
            let win32_err = ShellError::Win32("access denied".into());
            assert_eq!(format!("{win32_err}"), "win32 error: access denied");
        }
    }

    #[test]
    fn run_off_windows_or_invalid_scene() {
        let opts = RenderOptions::default();
        let res = run("non_existent_scene_xyz", opts);
        assert!(res.is_err());
    }

    #[test]
    fn autostart_stubs_or_impl() {
        #[cfg(not(windows))]
        {
            assert!(!autostart::is_installed());
            assert!(autostart::install("donut", "hacker").is_err());
            assert!(autostart::remove().is_err());
        }

        #[cfg(windows)]
        {
            let _ = autostart::is_installed();
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::ShellError;
    use aa_render::{render, RenderOptions};
    use std::sync::mpsc::{self, Sender};
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetDC, ReleaseDC, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetLastInputInfo, LASTINPUTINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumWindows,
        FindWindowExW, FindWindowW, GetSystemMetrics, PeekMessageW, PostQuitMessage,
        RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, SetWindowsHookExW, ShowWindow,
        TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LWA_COLORKEY, MSG, PM_REMOVE,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WH_KEYBOARD_LL, WM_DESTROY, WM_KEYDOWN,
        WM_QUIT, WM_SYSKEYDOWN, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_POPUP, WS_VISIBLE,
    };

    static DOOM_KEY_SENDER: OnceLock<Mutex<Option<Sender<Vec<u8>>>>> = OnceLock::new();

    fn win32<T>(r: windows::core::Result<T>) -> Result<T, ShellError> {
        r.map_err(|e| ShellError::Win32(e.message()))
    }

    pub fn run(
        scene_id: &str,
        opts: RenderOptions,
        doom_enabled: bool,
        settings: &[(String, f64)],
        idle_cycle: bool,
        capture_keys: bool,
    ) -> Result<(), ShellError> {
        // Virtual-screen geometry (spans all monitors).
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        let (vw, vh) = (vw.max(1), vh.max(1));

        let cols = (vw as u32 / opts.cell_w.max(1)).max(1) as usize;
        let rows = (vh as u32 / opts.cell_h.max(1)).max(1) as usize;
        let mut active_scene = scene_id.to_owned();
        let mut scene = start_scene(&active_scene, &opts, doom_enabled, settings, cols, rows)?;
        let scenes = if doom_enabled {
            vec!["donut", "helix", "matrix", "pipes", "life", "doom"]
        } else {
            vec!["donut", "helix", "matrix", "pipes", "life"]
        };
        let mut cycler = aa_core::idle_cycle::IdleCycle::default();
        let (key_events, keyboard_hook) = if capture_keys && doom_enabled {
            let (sender, receiver) = mpsc::channel();
            *DOOM_KEY_SENDER
                .get_or_init(|| Mutex::new(None))
                .lock()
                .unwrap() = Some(sender);
            let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0) }
                .map_err(|error| {
                ShellError::Win32(format!(
                    "could not enable DOOM key capture: {}",
                    error.message()
                ))
            })?;
            (Some(receiver), Some(hook))
        } else {
            (None, None)
        };

        let host = find_icon_host()?;
        let hwnd = create_render_window(host, vx, vy, vw, vh, !opts.opaque_background)?;
        if !opts.opaque_background {
            unsafe {
                let background = opts.theme.background;
                let key = COLORREF(
                    u32::from(background.r)
                        | (u32::from(background.g) << 8)
                        | (u32::from(background.b) << 16),
                );
                let _ = SetLayeredWindowAttributes(hwnd, key, 0, LWA_COLORKEY);
            }
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        let start = Instant::now();
        let mut next_idle_check = Instant::now();
        let mut idle_seconds = 0.0;
        let mut bgra: Vec<u8> = Vec::new();
        loop {
            // Drain the message queue without blocking the render loop.
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        scene.stop();
                        if let Some(hook) = keyboard_hook {
                            let _ = UnhookWindowsHookEx(hook);
                        }
                        if let Some(sender) = DOOM_KEY_SENDER.get() {
                            *sender.lock().unwrap() = None;
                        }
                        return Ok(());
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            let now = start.elapsed().as_secs_f64();
            if idle_cycle && Instant::now() >= next_idle_check {
                idle_seconds = system_idle_seconds();
                next_idle_check = Instant::now() + Duration::from_secs(5);
            }
            if idle_cycle {
                if let Some(next) = cycler.update(&active_scene, idle_seconds, now, &scenes) {
                    scene.stop();
                    scene = start_scene(&next, &opts, doom_enabled, settings, cols, rows)?;
                    active_scene = next;
                }
            }
            if let Some(events) = &key_events {
                while let Ok(bytes) = events.try_recv() {
                    if scene.is_interactive() {
                        scene.send_key(&bytes);
                    }
                }
            }
            let frame = scene.frame(now);
            let buf = render(&frame, &opts);
            blit(hwnd, &buf, &mut bgra, vw, vh)?;

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    fn start_scene(
        id: &str,
        opts: &RenderOptions,
        doom_enabled: bool,
        settings: &[(String, f64)],
        cols: usize,
        rows: usize,
    ) -> Result<Box<dyn aa_core::Scene + Send>, ShellError> {
        let mut scene = if id == "doom" {
            if !doom_enabled {
                return Err(ShellError::Win32(
                    "DOOM is opt-in. Enable it before selecting the scene.".into(),
                ));
            }
            Box::new(aa_doom::DoomScene::new(aa_doom::scaling_for_grid(
                cols, rows,
            ))) as Box<dyn aa_core::Scene + Send>
        } else {
            let mut scene = aa_core::scenes::make(id)
                .ok_or_else(|| ShellError::Win32(format!("unknown scene '{id}'")))?;
            for (setting, value) in settings {
                scene.apply_setting(setting, *value);
            }
            scene
        };
        scene.apply_base_color(opts.theme.text);
        scene.set_grid(cols, rows);
        scene.start();
        Ok(scene)
    }

    fn system_idle_seconds() -> f64 {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        unsafe {
            if GetLastInputInfo(&mut info).as_bool() {
                f64::from(GetTickCount().wrapping_sub(info.dwTime)) / 1000.0
            } else {
                0.0
            }
        }
    }

    unsafe extern "system" fn keyboard_hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code >= 0 && (message.0 == WM_KEYDOWN as usize || message.0 == WM_SYSKEYDOWN as usize) {
            let modifiers = [0x11, 0x12, 0x5b, 0x5c]
                .iter()
                .any(|key| GetAsyncKeyState(*key) < 0);
            if !modifiers {
                let key = (*(data.0 as *const KBDLLHOOKSTRUCT)).vkCode;
                if let Some(bytes) = doom_key_bytes(key) {
                    if let Some(sender) = DOOM_KEY_SENDER.get() {
                        if let Ok(sender) = sender.lock() {
                            if let Some(sender) = sender.as_ref() {
                                let _ = sender.send(bytes);
                            }
                        }
                    }
                }
            }
        }
        CallNextHookEx(None, code, message, data)
    }

    fn doom_key_bytes(key: u32) -> Option<Vec<u8>> {
        let sequence: &[u8] = match key {
            0x25 => b"\x1b[D",
            0x26 => b"\x1b[A",
            0x27 => b"\x1b[C",
            0x28 => b"\x1b[B",
            0x0d => b"\n",
            0x1b => b"\x1b",
            0x20 => b" ",
            0xbc => b",",
            0xbe => b".",
            0xdd => b"]",
            0x30..=0x39 | 0x41..=0x5a => {
                return Some(
                    vec![key as u8]
                        .into_iter()
                        .map(|byte| byte.to_ascii_lowercase())
                        .collect(),
                )
            }
            _ => return None,
        };
        Some(sequence.to_vec())
    }

    /// Find the top-level window that directly hosts `SHELLDLL_DefView` (the
    /// desktop icon layer). We use this as the Z-order anchor: our render
    /// window is inserted directly behind it so icons stay on top.
    /// Falls back to `Progman` if the icon host can't be found.
    fn find_icon_host() -> Result<HWND, ShellError> {
        unsafe {
            let progman = win32(FindWindowW(w!("Progman"), PCWSTR::null()))?;
            // NOTE: deliberately NOT sending WM_SPAWN_WORKERW (0x052C) here.
            // On Windows Server / RDP sessions that message causes the shell to
            // reorganise and hide SHELLDLL_DefView, making the icon layer vanish.
            // On Windows 10/11 desktop the spawned WorkerW is the correct parent,
            // but the Z-order anchor approach below works on both without it.
            let mut icon_host: HWND = HWND::default();
            let _ = EnumWindows(
                Some(enum_find_icon_host),
                LPARAM(&mut icon_host as *mut _ as isize),
            );
            if icon_host.is_invalid() {
                Ok(progman)
            } else {
                Ok(icon_host)
            }
        }
    }

    /// Enum callback: find whichever top-level window directly contains
    /// `SHELLDLL_DefView` (the desktop icon layer).
    unsafe extern "system" fn enum_find_icon_host(
        top: HWND,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let defview = FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null());
        if let Ok(dv) = defview {
            if !dv.is_invalid() {
                let out = lparam.0 as *mut HWND;
                *out = top;
            }
        }
        true.into()
    }

    fn create_render_window(
        parent: HWND,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        transparent: bool,
    ) -> Result<HWND, ShellError> {
        unsafe {
            let hinstance: HINSTANCE = win32(GetModuleHandleW(PCWSTR::null()))?.into();
            let class_name = w!("AsciiArcadeWallpaper");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: hinstance,
                lpszClassName: class_name,
                ..Default::default()
            };
            // Registering twice in one process is harmless; ignore a zero atom.
            RegisterClassW(&wc);

            // WS_POPUP (top-level, no parent) so we can place ourselves in the
            // global Z-order directly behind the icon host window. WS_CHILD would
            // lock our Z-order inside a parent that is itself above the icon layer.
            let ex_style = WS_EX_NOACTIVATE
                | WS_EX_TOOLWINDOW
                | if transparent {
                    WS_EX_LAYERED
                } else {
                    Default::default()
                };
            let hwnd = win32(CreateWindowExW(
                ex_style,
                class_name,
                w!("ASCII Arcade"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                w,
                h,
                None,
                None,
                Some(hinstance),
                None,
            ))?;
            // Insert directly behind the icon host in the global Z-order so
            // SHELLDLL_DefView (desktop icons) always renders on top of us.
            let _ = SetWindowPos(
                hwnd,
                Some(parent),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            Ok(hwnd)
        }
    }

    extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
    }

    /// Blit an [`aa_render`] RGBA buffer to the window, stretching it to the
    /// virtual-screen rect. GDI 32bpp DIBs are BGRA in memory, so we swizzle into
    /// a reused scratch buffer and use a top-down (negative-height) bitmap.
    fn blit(
        hwnd: HWND,
        buf: &aa_render::PixelBuffer,
        bgra: &mut Vec<u8>,
        dst_w: i32,
        dst_h: i32,
    ) -> Result<(), ShellError> {
        bgra.resize(buf.pixels.len(), 0);
        for (dst, src) in bgra
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(buf.pixels.as_chunks::<4>().0.iter())
        {
            dst[0] = src[2]; // B
            dst[1] = src[1]; // G
            dst[2] = src[0]; // R
            dst[3] = src[3]; // A (ignored by BI_RGB)
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: buf.width as i32,
                biHeight: -(buf.height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            let hdc = GetDC(Some(hwnd));
            if hdc.is_invalid() {
                return Err(ShellError::Win32("GetDC returned null".into()));
            }
            let scanned = StretchDIBits(
                hdc,
                0,
                0,
                dst_w,
                dst_h,
                0,
                0,
                buf.width as i32,
                buf.height as i32,
                Some(bgra.as_ptr() as *const _),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(Some(hwnd), hdc);
            if scanned == 0 {
                return Err(ShellError::Win32("StretchDIBits drew 0 scanlines".into()));
            }
        }
        Ok(())
    }
}

/// Manage the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry that
/// tells Windows to launch `aa-windows` at user login.
pub mod autostart {
    /// Register `aa-windows` as a Windows login item.
    /// `scene` and `theme` are passed through as CLI args to the launched process.
    pub fn install(scene: &str, theme: &str) -> Result<(), String> {
        install_with_doom(scene, theme, false)
    }

    pub fn install_with_doom(scene: &str, theme: &str, doom_enabled: bool) -> Result<(), String> {
        let args = if doom_enabled {
            vec!["--enable-doom".to_owned()]
        } else {
            Vec::new()
        };
        install_with_args(scene, theme, &args)
    }

    pub fn install_with_args(scene: &str, theme: &str, args: &[String]) -> Result<(), String> {
        #[cfg(windows)]
        {
            imp::install(scene, theme, args)
        }
        #[cfg(not(windows))]
        {
            let _ = (scene, theme, args);
            Err("autostart is only supported on Windows".into())
        }
    }

    /// Remove the `aa-windows` login item, if present.
    pub fn remove() -> Result<(), String> {
        #[cfg(windows)]
        {
            imp::remove()
        }
        #[cfg(not(windows))]
        {
            Err("autostart is only supported on Windows".into())
        }
    }

    /// Returns `true` if the login-item registry value is currently present.
    pub fn is_installed() -> bool {
        #[cfg(windows)]
        {
            imp::is_installed()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    #[cfg(windows)]
    mod imp {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::WIN32_ERROR;
        use windows::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
            RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        };

        const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        const APP_VALUE: &str = "AsciiArcade";
        const ERROR_SUCCESS: WIN32_ERROR = WIN32_ERROR(0u32);

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0u16)).collect()
        }

        pub fn install(scene: &str, theme: &str, args: &[String]) -> Result<(), String> {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let extra = args.join(" ");
            let suffix = if extra.is_empty() {
                String::new()
            } else {
                format!(" {extra}")
            };
            let cmd = format!("\"{}\" {} {}{}", exe.display(), scene, theme, suffix);

            let key_wide = to_wide(RUN_KEY);
            let value_wide = to_wide(APP_VALUE);
            let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0u16)).collect();
            let cmd_bytes: Vec<u8> = cmd_wide.iter().flat_map(|&c| c.to_le_bytes()).collect();

            unsafe {
                let mut hkey = HKEY::default();
                let err = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key_wide.as_ptr()),
                    Some(0),
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_SET_VALUE,
                    None,
                    &mut hkey,
                    None,
                );
                if err != ERROR_SUCCESS {
                    return Err(format!("RegCreateKeyExW failed: 0x{:08x}", err.0));
                }
                let set_err = RegSetValueExW(
                    hkey,
                    PCWSTR(value_wide.as_ptr()),
                    Some(0),
                    REG_SZ,
                    Some(cmd_bytes.as_slice()),
                );
                let _ = RegCloseKey(hkey);
                if set_err != ERROR_SUCCESS {
                    return Err(format!("RegSetValueExW failed: 0x{:08x}", set_err.0));
                }
            }
            Ok(())
        }

        pub fn remove() -> Result<(), String> {
            let key_wide = to_wide(RUN_KEY);
            let value_wide = to_wide(APP_VALUE);
            unsafe {
                let mut hkey = HKEY::default();
                let err = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key_wide.as_ptr()),
                    Some(0),
                    KEY_SET_VALUE,
                    &mut hkey,
                );
                if err == ERROR_SUCCESS {
                    let _ = RegDeleteValueW(hkey, PCWSTR(value_wide.as_ptr()));
                    let _ = RegCloseKey(hkey);
                }
            }
            Ok(())
        }

        pub fn is_installed() -> bool {
            let key_wide = to_wide(RUN_KEY);
            let value_wide = to_wide(APP_VALUE);
            unsafe {
                let mut hkey = HKEY::default();
                let err = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key_wide.as_ptr()),
                    Some(0),
                    KEY_QUERY_VALUE,
                    &mut hkey,
                );
                if err != ERROR_SUCCESS {
                    return false;
                }
                let q = RegQueryValueExW(hkey, PCWSTR(value_wide.as_ptr()), None, None, None, None);
                let _ = RegCloseKey(hkey);
                q == ERROR_SUCCESS
            }
        }
    }
}
