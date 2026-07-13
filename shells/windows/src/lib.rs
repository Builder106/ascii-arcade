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
    #[cfg(windows)]
    {
        imp::run(scene_id, opts)
    }
    #[cfg(not(windows))]
    {
        let _ = (scene_id, opts);
        Err(ShellError::Unsupported)
    }
}

#[cfg(windows)]
mod imp {
    use super::ShellError;
    use aa_render::{render, RenderOptions};
    use std::time::Instant;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetDC, ReleaseDC, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumWindows, FindWindowExW, FindWindowW,
        GetSystemMetrics, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowPos, ShowWindow,
        TranslateMessage, HMENU, MSG, PM_REMOVE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW,
        WM_DESTROY, WM_QUIT, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
    };

    fn win32<T>(r: windows::core::Result<T>) -> Result<T, ShellError> {
        r.map_err(|e| ShellError::Win32(e.message()))
    }

    pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
        let mut scene = aa_core::scenes::make(scene_id)
            .ok_or_else(|| ShellError::Win32(format!("unknown scene '{scene_id}'")))?;
        // Colour scenes (Matrix, …) key their palette off the theme colour.
        scene.apply_base_color(opts.theme.text);

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

        let host = find_icon_host()?;
        let hwnd = create_render_window(host, vx, vy, vw, vh)?;
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        // Size the scene's character grid to the virtual screen / cell metrics.
        let cols = (vw as u32 / opts.cell_w.max(1)).max(1) as usize;
        let rows = (vh as u32 / opts.cell_h.max(1)).max(1) as usize;
        scene.set_grid(cols, rows);
        scene.start();

        let start = Instant::now();
        let mut bgra: Vec<u8> = Vec::new();
        loop {
            // Drain the message queue without blocking the render loop.
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        scene.stop();
                        return Ok(());
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            let frame = scene.frame(start.elapsed().as_secs_f64());
            let buf = render(&frame, &opts);
            blit(hwnd, &buf, &mut bgra, vw, vh)?;

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
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
    ) -> windows::Win32::Foundation::BOOL {
        let defview = FindWindowExW(top, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null());
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
            let hwnd = win32(CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class_name,
                w!("ASCII Arcade"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                w,
                h,
                HWND::default(),
                HMENU::default(),
                hinstance,
                None,
            ))?;
            // Insert directly behind the icon host in the global Z-order so
            // SHELLDLL_DefView (desktop icons) always renders on top of us.
            let _ = SetWindowPos(
                hwnd,
                parent,
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
        for (dst, src) in bgra.chunks_exact_mut(4).zip(buf.pixels.chunks_exact(4)) {
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
            let hdc = GetDC(hwnd);
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
            ReleaseDC(hwnd, hdc);
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
        #[cfg(windows)]
        {
            imp::install(scene, theme)
        }
        #[cfg(not(windows))]
        {
            let _ = (scene, theme);
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

        pub fn install(scene: &str, theme: &str) -> Result<(), String> {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let cmd = format!("\"{}\" {} {}", exe.display(), scene, theme);

            let key_wide = to_wide(RUN_KEY);
            let value_wide = to_wide(APP_VALUE);
            let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0u16)).collect();
            let cmd_bytes: Vec<u8> = cmd_wide.iter().flat_map(|&c| c.to_le_bytes()).collect();

            unsafe {
                let mut hkey = HKEY::default();
                let err = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key_wide.as_ptr()),
                    0,
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
                    0,
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
                    0,
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
                    0,
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
