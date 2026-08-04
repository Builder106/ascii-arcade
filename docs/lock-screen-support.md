# Lock screen support

> Status: exploratory — no build started. This is a feasibility survey, not a
> commitment. Android is real, ready-to-schedule work; the other four
> platforms need a research spike before any implementation plan gets written.

## Current state

- **Android** (`shells/android/app/src/main/kotlin/.../wallpaper/AaWallpaperService.kt`) —
  implements `WallpaperService`, registered in `AndroidManifest.xml` with
  `android:permission="android.permission.BIND_WALLPAPER"` and declared via
  `res/xml/wallpaper.xml`. This is the standard Android live-wallpaper engine
  contract; the OS's wallpaper picker already lets a user apply any live
  wallpaper to the home screen, the lock screen, or both.
- **macOS** — the `AsciiArcade` AppKit host paints an `NSWindow` positioned
  behind the desktop icons (a window-level trick, not a public wallpaper API).
  Scenes render straight onto the desktop behind windows.
- **Windows** (`shells/windows/src/lib.rs`) — the **WorkerW** technique: sends
  the undocumented `0x052C` message to `Progman`, which spawns a `WorkerW`
  window behind the desktop icons; enumerates top-level windows to find it,
  `SetParent`s the render window into it, and blits each frame with GDI
  `StretchDIBits`. Same trick Lively Wallpaper uses.
- **Linux** (`shells/linux/src/lib.rs`) — backend picked at runtime from
  `WAYLAND_DISPLAY`: **X11** paints a root-window pixmap and publishes it via
  the `_XROOTPMAP_ID`/`ESETROOT_PMAP_ID` convention (what feh/conky use, and
  what covers XWayland sessions too); **Wayland** opens a `wlr-layer-shell`
  surface on the `background` layer, which works on wlroots compositors (sway,
  Hyprland) and KDE but explicitly excludes GNOME-Wayland (needs a Shell
  extension).
- **iOS** — SwiftUI/Metal `shells/ios/AsciiArcade/`, not yet reviewed for any
  lock-screen-adjacent surface (WidgetKit lock-screen widgets, StandBy). That
  review is part of this platform's spike, not assumed here.

## Per-platform feasibility

### Android — real work, not a spike

The engine already runs as a `WallpaperService`; lock-screen placement is a
call the *system* makes on the user's behalf (`WallpaperManager.setBitmap` /
`setStream` with `FLAG_LOCK` vs `FLAG_SYSTEM`, or the OS wallpaper picker's
"Home screen / Lock screen / Both" prompt), not something the engine has to
implement from scratch. Confirm live (not just static) rendering is honored on
the lock screen on a couple of OEM skins (Pixel stock, Samsung One UI) — some
manufacturers downgrade lock-screen live wallpapers to a single static frame
for battery reasons, which would be a discoverable-at-test-time constraint,
not a design question.

### macOS — spike needed, likely dead end

The desktop host draws into a window at the desktop-icon layer, which is a
different security domain from the Lock Screen. The Lock Screen is rendered by
`loginwindow`/the login/lock compositor, a separate secure process that
doesn't load third-party window content — this is the same boundary that
keeps password-prompt UI tamper-proof. Apple does let users sync their desktop
picture to the lock screen (System Settings → Wallpaper, "Add a New Wallpaper"
flow) but that's a *static image handoff* through a public preference, not a
live rendering surface.

**Spike questions:**

- Does `NSWorkspace`/`SystemConfiguration` expose any documented API to push a
  static image to the lock screen's wallpaper slot (as opposed to the desktop
  picture, which already syncs via System Settings)?
- Is there prior art (open-source or write-ups) of any third-party macOS app
  achieving *animated* lock-screen content, or is static-image-only the
  ceiling industry-wide?

### Windows — spike needed, likely dead end

The WorkerW trick places a window behind desktop icons — a desktop-session
concept. The actual Windows Lock Screen is a UWP-only compositor surface
(`Windows.System.UserProfile.LockScreen` namespace), which accepts a **static
image path or the Windows Spotlight/slideshow provider**, and only from a
UWP-packaged app, not from a Win32 app using GDI/WorkerW.

**Spike questions:**

- Does `LockScreen.SetImageFileAsync` (or the older `Windows.System.UserProfile`
  APIs) let a lock-screen image be swapped periodically/programmatically from
  outside a UWP context (e.g. Win32 app with the right capability
  declarations), or does packaging as UWP/MSIX become a hard requirement?
- If MSIX packaging is required, is that worth taking on for a periodically-
  refreshed static image, given the engine's identity is live rendering?

### Linux — spike needed, fragmented by design

There's no unified lock-screen API on Linux at all — whichever greeter/locker
owns the seat (`gdm`, `sddm`, `swaylock`, `hyprlock`, etc.) decides what it
will render, and most only accept a static image path via their own config
format, not a rendering process.

**Spike questions:**

- For the two backends already supported (X11 desktop, wlroots-Wayland
  desktop), what are the two or three most common locker pairings in that
  same session type (e.g. sway + swaylock, Hyprland + hyprlock), and do any of
  them support periodic external image refresh (a cron-style `swaylock -i
  <path>` re-invocation, or a config `reload` signal)?
- Is a "generate a static ASCII-art frame, hand it to whatever locker is
  configured" integration worth building, given it would be locker-specific
  glue code rather than a shared engine feature?

### iOS — spike needed, scope depends on WidgetKit ceiling

Apple doesn't allow third-party live/animated Lock Screen backgrounds — only
Apple's own Depth Effect and Photo Shuffle qualify for that surface. The
adjacent public surface is **Lock Screen widgets** (WidgetKit,
`.accessoryCircular`/`.accessoryRectangular`/`.accessoryInline` families),
which are small, mostly monochrome, and refresh on a budgeted timeline
(`TimelineProvider`), not full-screen or continuously animated.

**Spike questions:**

- Would a Lock Screen *widget* rendering a tiny glyph-scale animation frame
  (e.g. a single Matrix-rain column, a donut reduced to a handful of glyphs)
  be recognizably "ASCII Arcade" at that size, or is the format too
  constrained to be worth it?
- What does WidgetKit's refresh budget realistically allow — is "looks
  animated" achievable within the system's timeline-entry limits, or does it
  read as a slow slideshow?

## Recommendation

- **Ship it**: Android. The engine already qualifies; this is a
  flag/entitlement change plus OEM spot-checks, not new architecture.
- **Spike, then decide**: iOS Lock Screen widget (most promising of the four —
  Apple ships a real, if narrow, public API for it) and Linux (fragmented but
  at least has *a* path per compositor pairing already in scope).
- **Spike, expect dead end**: macOS and Windows. Both would require
  discovering an undocumented or newly-shipped OS hook to beat the
  static-image-only ceiling; budget the spike as "confirm there's still no
  way," not "find the way."

## Next steps

Each spike is small enough to run independently and does not block the
others:

1. **iOS**: build a throwaway `WidgetKit` lock-screen widget target, confirm
   whether a recognizable single-glyph-scale animation reads at that size and
   refresh cadence.
2. **Linux**: pick one X11 and one Wayland locker pairing already common among
   the two supported session types, confirm whether either accepts a
   periodically-refreshed static image without forking the locker itself.
3. **macOS**: check current System Settings/`NSWorkspace` documentation (and
   recent WWDC sessions, since Apple has expanded Lock Screen customization in
   recent macOS releases) for any public hook beyond desktop-picture sync.
4. **Windows**: check current `Windows.System.UserProfile.LockScreen` docs for
   whether Win32 (non-MSIX) callers can set a lock-screen image, and whether
   that's changed since Windows 11's Lock Screen redesign.

Each spike that finds a real path gets its own design doc; each that confirms
a dead end gets a one-paragraph note here so this doesn't get re-investigated
later without new information.
