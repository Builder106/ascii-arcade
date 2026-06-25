//! # aa-windows
//!
//! The Windows wallpaper shell. Uses the **WorkerW** technique: send the
//! undocumented `0x052C` message to `Progman`, which spawns a `WorkerW` window
//! *behind* the desktop icons; enumerate windows to find it, then `SetParent`
//! our render window into it. Render = blit [`aa_render`] output via Direct2D.
//! (Lively Wallpaper is the open-source C# reference for the same trick.)
//!
//! STATUS: skeleton. Real implementation is `#[cfg(windows)]` and built on CI
//! (`windows-latest`); off-Windows this crate is an empty stub so the workspace
//! checks on macOS/Linux.

/// Errors the shell can surface to its caller.
#[derive(Debug)]
pub enum ShellError {
    Unsupported,
    #[cfg(windows)]
    Win32(String),
}

/// Launch the wallpaper host: find/own the WorkerW surface and run the render
/// loop until exit.
pub fn run() -> Result<(), ShellError> {
    #[cfg(windows)]
    {
        imp::run()
    }
    #[cfg(not(windows))]
    {
        Err(ShellError::Unsupported)
    }
}

#[cfg(windows)]
mod imp {
    use super::ShellError;

    // TODO(aa-windows):
    //   1. find_worker_w(): SendMessageTimeout(progman, 0x052C, ...) then
    //      EnumWindows for the WorkerW that owns the wallpaper layer.
    //   2. create a render HWND, SetParent it into the WorkerW.
    //   3. Direct2D device + per-monitor swapchain; blit aa_render::render().
    //   4. tray icon (scene/theme switch) + low-level keyboard hook for DOOM.
    pub fn run() -> Result<(), ShellError> {
        Err(ShellError::Win32("not yet implemented".into()))
    }
}
