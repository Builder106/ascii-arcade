# Opaque background toggle for Hacker/Amber/Ice themes

## Problem

Ghost is the only theme with an opaque backing (`Theme.backgroundColor` set to
a near-white). Hacker, Amber, and Ice all have `backgroundColor: nil`, so the
scene view's layer stays transparent and the real desktop wallpaper shows
through behind the glyphs. The request: give those three themes a solid black
backing too — visually, "the wallpaper goes black while the scene is open" —
while letting people opt back into the transparent look if they prefer it,
since that's a legitimate creative choice some users will want to keep.

## Approach

Purely a `SceneView`-layer concern. No `NSWorkspace`/`setWallpaper` call is
involved — the desktop-level scene window already paints its own backing on
top of the real wallpaper, so making that backing opaque achieves the same
visual result ("black when open, real wallpaper... well, never actually
touched, so nothing to restore") without ever touching
`NSWorkspace.shared.setDesktopImageURL`. The existing `originalWallpapers`
save/restore machinery in `AppDelegate` (lines ~671, 696–699, 791–794) is
unrelated to this feature and is left untouched — it's already dead code
today (nothing currently diverges the real wallpaper from what's saved), and
fixing that is out of scope here.

### 1. Themes default to opaque

```swift
let availableThemes: [Theme] = [
    Theme(name: "Hacker", textColor: .systemGreen, backgroundColor: .black),
    Theme(name: "Amber", textColor: NSColor(calibratedRed: 1.0, green: 0.65, blue: 0.0, alpha: 1.0), backgroundColor: .black),
    Theme(name: "Ice", textColor: .cyan, backgroundColor: .black),
    Theme(name: "Ghost", textColor: NSColor(calibratedRed: 0.11, green: 0.11, blue: 0.118, alpha: 1.0),
          backgroundColor: NSColor(calibratedRed: 0.961, green: 0.961, blue: 0.961, alpha: 1.0)),
]
```

### 2. New menu toggle: "Opaque Background"

- A checkable item in the `◎` status menu, alongside the existing
  Scene/Theme/Settings items. On by default.
- When off, the effective background for *any* theme (including Ghost) is
  forced to `nil`/transparent regardless of what `availableThemes` specifies
  — so a user who wants the classic "real desktop through the glyphs" look
  gets it uniformly, not just for the three new themes.
- Persisted the same way other prefs are: add `opaqueBackground: Bool`
  (default `true`) to the saved-state struct that `loadState()` /
  `sceneSettingSelections` already populate, restored on launch.
- Toggling routes through the existing `view.applyTheme(...)` call path (line
  ~1139) so it takes effect immediately on all screens without restarting the
  active scene.

## Data flow

1. Launch: `loadState()` restores `opaqueBackground` (default `true` if no
   saved state).
2. `applyTheme(availableThemes[currentThemeIndex])` is called per-view at
   startup; the effective background it applies is
   `opaqueBackground ? theme.backgroundColor : nil`.
3. Menu toggle flips `opaqueBackground`, persists it, and re-invokes
   `applyTheme` on all views with the same effective-background logic.
4. Theme switching (existing `cycleScenes`/theme picker path) continues to
   call `applyTheme` as it does today; the toggle's effect composes with it
   automatically since both funnel through the same effective-background
   computation.

## Out of scope

- Any change to `setWallpaper`/`originalWallpapers`/`NSWorkspace` — the real
  macOS desktop picture is never read or written by this feature.
- Per-theme custom colors beyond "opaque theme color vs. transparent" (e.g. no
  color picker).
- Cross-platform work (Windows/Linux/iOS/Android) — this app is macOS-only.
