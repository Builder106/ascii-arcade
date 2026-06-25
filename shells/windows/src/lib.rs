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
        GetSystemMetrics, PeekMessageW, PostQuitMessage, RegisterClassW, SendMessageTimeoutW,
        SetParent, ShowWindow, TranslateMessage, CW_USEDEFAULT, HMENU, MSG, PM_REMOVE, SMTO_NORMAL,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW,
        WM_DESTROY, WM_QUIT, WNDCLASSW, WS_CHILD, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_VISIBLE,
    };

    fn win32<T>(r: windows::core::Result<T>) -> Result<T, ShellError> {
        r.map_err(|e| ShellError::Win32(e.message()))
    }

    /// The desktop's "spawn a WorkerW behind the icons" magic message.
    const WM_SPAWN_WORKERW: u32 = 0x052C;

    pub fn run(scene_id: &str, opts: RenderOptions) -> Result<(), ShellError> {
        let mut scene = aa_core::scenes::make(scene_id)
            .ok_or_else(|| ShellError::Win32(format!("unknown scene '{scene_id}'")))?;
        // Colour scenes (Matrix, fire, …) key their palette off the theme colour.
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

        let host = find_wallpaper_host()?;
        let hwnd = create_render_window(vx, vy, vw, vh)?;
        unsafe {
            // Reparent into the WorkerW so we render behind the icons.
            let _ = SetParent(hwnd, host);
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

    /// Find the WorkerW that hosts the wallpaper, spawning it if needed. Falls
    /// back to `Progman` itself (some shells paint the wallpaper there directly).
    fn find_wallpaper_host() -> Result<HWND, ShellError> {
        unsafe {
            let progman = win32(FindWindowW(w!("Progman"), PCWSTR::null()))?;
            // Ask the desktop to create the WorkerW layer. Ignore the result:
            // on shells that don't, we fall back to Progman below.
            let _ = SendMessageTimeoutW(
                progman,
                WM_SPAWN_WORKERW,
                WPARAM(0),
                LPARAM(0),
                SMTO_NORMAL,
                1000,
                None,
            );

            let mut found: HWND = HWND::default();
            let _ = EnumWindows(
                Some(enum_find_workerw),
                LPARAM(&mut found as *mut _ as isize),
            );
            if found.is_invalid() {
                Ok(progman)
            } else {
                Ok(found)
            }
        }
    }

    /// Top-level enum callback: the wallpaper WorkerW is the sibling that comes
    /// right after the window hosting `SHELLDLL_DefView` (the icon layer).
    unsafe extern "system" fn enum_find_workerw(
        top: HWND,
        lparam: LPARAM,
    ) -> windows::Win32::Foundation::BOOL {
        let defview = FindWindowExW(top, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null());
        if let Ok(dv) = defview {
            if !dv.is_invalid() {
                if let Ok(worker) =
                    FindWindowExW(HWND::default(), top, w!("WorkerW"), PCWSTR::null())
                {
                    if !worker.is_invalid() {
                        let out = lparam.0 as *mut HWND;
                        *out = worker;
                    }
                }
            }
        }
        true.into()
    }

    fn create_render_window(x: i32, y: i32, w: i32, h: i32) -> Result<HWND, ShellError> {
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

            let hwnd = win32(CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class_name,
                w!("ASCII Arcade"),
                WS_CHILD | WS_VISIBLE,
                if x == 0 { CW_USEDEFAULT } else { x },
                y,
                w,
                h,
                HWND::default(),
                HMENU::default(),
                hinstance,
                None,
            ))?;
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
