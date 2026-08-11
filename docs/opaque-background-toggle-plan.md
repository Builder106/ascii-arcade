# Opaque Background Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Hacker/Amber/Ice themes an opaque black backing (matching Ghost's existing white), with a menu toggle to fall back to the transparent "real desktop shows through" look for any theme.

**Architecture:** Pure `SceneView`-layer change — the desktop-level scene window already paints its own backing above the real wallpaper (see `Sources/AsciiArcade/main.swift:55-67`), so making that backing opaque needs no `NSWorkspace`/`setWallpaper` involvement. `SceneView` gains an `opaqueBackgroundEnabled` flag that gates whether `Theme.backgroundColor` is actually applied to the layer. `AppDelegate` owns the persisted on/off state and a new "Opaque Background" menu item, following the exact pattern already used for `idleAutoCycle`/`toggleIdleAutoCycle`.

**Tech Stack:** Swift 5.10, AppKit (`NSView`, `NSMenuItem`, `CALayer`), `Codable`/`UserDefaults` for persistence — no new dependencies.

## Global Constraints

- macOS 13+ target (`Package.swift` platforms) — no APIs newer than that.
- No `NSWorkspace.shared.setDesktopImageURL`/`desktopImageURL` calls — this feature never reads or writes the real desktop picture (spec: "Out of scope").
- No new persisted-state key breaks existing saved state: use the same `decodeIfPresent(...) ?? default` backward-compatible pattern already used for `doomEnabled` (`main.swift:510`).
- This target has no unit test coverage for `AppDelegate`/`SceneView` (only `AsciiArcadeCore` and `Hotword` have `Tests/` targets) — verification here is `swift build` for compilation plus a manual launch check, matching how `idleAutoCycle`/`doomEnabled` were verified. Do not invent a fake XCTest target for AppKit UI code the rest of the file doesn't test either.

---

### Task 1: Themes default to opaque black

**Files:**
- Modify: `Sources/AsciiArcade/main.swift:16-22`

**Interfaces:**
- Produces: `availableThemes[0..2].backgroundColor` now `.black` instead of `nil` (Ghost, index 3, unchanged).

- [ ] **Step 1: Change the theme table**

Replace lines 16-22:

```swift
let availableThemes: [Theme] = [
    Theme(name: "Hacker", textColor: .systemGreen, backgroundColor: nil),
    Theme(name: "Amber", textColor: NSColor(calibratedRed: 1.0, green: 0.65, blue: 0.0, alpha: 1.0), backgroundColor: nil),
    Theme(name: "Ice", textColor: .cyan, backgroundColor: nil),
    Theme(name: "Ghost", textColor: NSColor(calibratedRed: 0.11, green: 0.11, blue: 0.118, alpha: 1.0),
          backgroundColor: NSColor(calibratedRed: 0.961, green: 0.961, blue: 0.961, alpha: 1.0)),
]
```

with:

```swift
let availableThemes: [Theme] = [
    Theme(name: "Hacker", textColor: .systemGreen, backgroundColor: .black),
    Theme(name: "Amber", textColor: NSColor(calibratedRed: 1.0, green: 0.65, blue: 0.0, alpha: 1.0), backgroundColor: .black),
    Theme(name: "Ice", textColor: .cyan, backgroundColor: .black),
    Theme(name: "Ghost", textColor: NSColor(calibratedRed: 0.11, green: 0.11, blue: 0.118, alpha: 1.0),
          backgroundColor: NSColor(calibratedRed: 0.961, green: 0.961, blue: 0.961, alpha: 1.0)),
]
```

- [ ] **Step 2: Build to verify it compiles**

Run: `swift build`
Expected: build succeeds (no test target exercises `main.swift`, so this is the verification step for this task).

- [ ] **Step 3: Commit**

```bash
git add Sources/AsciiArcade/main.swift
git commit -m "Give Hacker/Amber/Ice themes an opaque black background"
```

---

### Task 2: SceneView gates the background behind an opaqueBackgroundEnabled flag

**Files:**
- Modify: `Sources/AsciiArcade/main.swift:81` (add stored property), `main.swift:190-196` (`applyTheme`)

**Interfaces:**
- Consumes: nothing new from Task 1 beyond the theme table already in place.
- Produces: `SceneView.opaqueBackgroundEnabled: Bool` (default `true`), `SceneView.setOpaqueBackgroundEnabled(_ enabled: Bool)` — later tasks (3, 4) call this on every view in `AppDelegate.views`.

- [ ] **Step 1: Add the stored theme + flag properties**

At `main.swift:81`, after the existing `themeTextColor` line, add two properties (the view doesn't currently retain the full `Theme` it was given — only the text color — so `applyTheme` has nothing to recompute the background from when the flag changes; store it):

```swift
    private var themeTextColor: NSColor = availableThemes[0].textColor
    private var currentTheme: Theme = availableThemes[0]
    private var opaqueBackgroundEnabled = true
```

- [ ] **Step 2: Route `applyTheme` through an effective-background helper**

Replace `main.swift:190-196`:

```swift
    func applyTheme(_ theme: Theme) {
        themeTextColor = theme.textColor
        layer?.backgroundColor = theme.backgroundColor?.cgColor ?? NSColor.clear.cgColor
        let rgb = SceneView.rgbColor(from: theme.textColor)
        for scene in scenes { scene.applyBaseColor(rgb) }
        needsDisplay = true
    }
```

with:

```swift
    func applyTheme(_ theme: Theme) {
        currentTheme = theme
        themeTextColor = theme.textColor
        layer?.backgroundColor = effectiveBackgroundColor
        let rgb = SceneView.rgbColor(from: theme.textColor)
        for scene in scenes { scene.applyBaseColor(rgb) }
        needsDisplay = true
    }

    /// The "Opaque Background" menu toggle overrides every theme's own
    /// `backgroundColor` to `nil` when off, so turning it off restores the
    /// classic "real desktop shows through the glyphs" look uniformly —
    /// including for Ghost, not just the newly-opaque Hacker/Amber/Ice.
    private var effectiveBackgroundColor: CGColor {
        (opaqueBackgroundEnabled ? currentTheme.backgroundColor : nil)?.cgColor ?? NSColor.clear.cgColor
    }

    /// Flip the opaque/transparent backing without touching the active scene
    /// or text color — called from the menu toggle in `AppDelegate`.
    func setOpaqueBackgroundEnabled(_ enabled: Bool) {
        opaqueBackgroundEnabled = enabled
        layer?.backgroundColor = effectiveBackgroundColor
        needsDisplay = true
    }
```

- [ ] **Step 3: Build to verify it compiles**

Run: `swift build`
Expected: build succeeds. (`setOpaqueBackgroundEnabled` is unused until Task 4 wires it up — Swift doesn't warn on unused public/internal methods, only unused locals, so this won't produce a warning.)

- [ ] **Step 4: Commit**

```bash
git add Sources/AsciiArcade/main.swift
git commit -m "Add SceneView opaque-background toggle plumbing"
```

---

### Task 3: Persist the opaqueBackground preference and restore it at launch

**Files:**
- Modify: `Sources/AsciiArcade/main.swift:482-513` (`PersistedState`), `main.swift:664-733` (`AppDelegate` properties + `applicationDidFinishLaunching`), `main.swift:1074-1087` (`saveState`)

**Interfaces:**
- Consumes: `SceneView.setOpaqueBackgroundEnabled(_:)` from Task 2.
- Produces: `AppDelegate.opaqueBackground: Bool` (default `true`), applied to every view before `saveState()` can be called on it — Task 4's menu handler reads/writes this same property.

- [ ] **Step 1: Add the field to `PersistedState`, with backward-compatible decode**

In `main.swift:482-513`, add `opaqueBackground` alongside `doomEnabled`, following the exact pattern that field already uses so state saved before this change still loads:

```swift
struct PersistedState: Codable {
    var sceneIndex: Int
    var themeIndex: Int
    var captureKeysForDoom: Bool
    var idleAutoCycle: Bool
    var doomEnabled: Bool
    var opaqueBackground: Bool
    /// scene index (as String for JSON) → (settingId → chosen option index).
    var sceneSettings: [String: [String: Int]]

    init(sceneIndex: Int, themeIndex: Int, captureKeysForDoom: Bool, idleAutoCycle: Bool,
         doomEnabled: Bool, opaqueBackground: Bool, sceneSettings: [String: [String: Int]]) {
        self.sceneIndex = sceneIndex
        self.themeIndex = themeIndex
        self.captureKeysForDoom = captureKeysForDoom
        self.idleAutoCycle = idleAutoCycle
        self.doomEnabled = doomEnabled
        self.opaqueBackground = opaqueBackground
        self.sceneSettings = sceneSettings
    }

    /// Custom decode so state saved before `doomEnabled`/`opaqueBackground`
    /// existed still loads — missing keys default to their pre-feature
    /// behavior (`doomEnabled` off, `opaqueBackground` on, since opaque is
    /// the new default) instead of discarding the whole saved state on every
    /// returning user.
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        sceneIndex = try c.decode(Int.self, forKey: .sceneIndex)
        themeIndex = try c.decode(Int.self, forKey: .themeIndex)
        captureKeysForDoom = try c.decode(Bool.self, forKey: .captureKeysForDoom)
        idleAutoCycle = try c.decode(Bool.self, forKey: .idleAutoCycle)
        doomEnabled = try c.decodeIfPresent(Bool.self, forKey: .doomEnabled) ?? false
        opaqueBackground = try c.decodeIfPresent(Bool.self, forKey: .opaqueBackground) ?? true
        sceneSettings = try c.decode([String: [String: Int]].self, forKey: .sceneSettings)
    }
}
```

- [ ] **Step 2: Add the `AppDelegate` property**

Near `main.swift:685` (`var idleAutoCycle = false`), add:

```swift
    var opaqueBackground = true
```

- [ ] **Step 3: Restore it in `applicationDidFinishLaunching` and apply it to every view**

In the `if let s = restored { ... }` block (around `main.swift:704-711`), add the restore line alongside the others:

```swift
        if let s = restored {
            currentSceneIndex = min(max(0, s.sceneIndex), sceneNames.count - 1)
            currentThemeIndex = min(max(0, s.themeIndex), availableThemes.count - 1)
            captureKeysForDoom = s.captureKeysForDoom
            idleAutoCycle = s.idleAutoCycle
            doomEnabled = s.doomEnabled
            opaqueBackground = s.opaqueBackground
            sceneSettingSelections = Dictionary(uniqueKeysWithValues:
                s.sceneSettings.compactMap { key, val in Int(key).map { ($0, val) } })
        }
```

Then in the per-screen setup loop (around `main.swift:720-733`), apply it right after `applyTheme` so the very first frame already reflects the restored preference:

```swift
            view.applyPersistedSettings(sceneSettingSelections)
            view.applyTheme(availableThemes[currentThemeIndex])
            view.setOpaqueBackgroundEnabled(opaqueBackground)
            view.selectScene(currentSceneIndex)
```

- [ ] **Step 4: Include it in `saveState`**

In `main.swift:1074-1087`, add `opaqueBackground: opaqueBackground` to the `PersistedState(...)` call:

```swift
    private func saveState() {
        let sceneSettings = Dictionary(uniqueKeysWithValues:
            sceneSettingSelections.map { (String($0.key), $0.value) })
        let state = PersistedState(
            sceneIndex: currentSceneIndex,
            themeIndex: currentThemeIndex,
            captureKeysForDoom: captureKeysForDoom,
            idleAutoCycle: idleAutoCycle,
            doomEnabled: doomEnabled,
            opaqueBackground: opaqueBackground,
            sceneSettings: sceneSettings)
        if let data = try? JSONEncoder().encode(state) {
            UserDefaults.standard.set(data, forKey: stateKey)
        }
    }
```

- [ ] **Step 5: Build to verify it compiles**

Run: `swift build`
Expected: build succeeds. All `PersistedState(...)` call sites now pass `opaqueBackground` (there is exactly one, in `saveState`), and `init(from:)` handles the one JSON-decode path.

- [ ] **Step 6: Commit**

```bash
git add Sources/AsciiArcade/main.swift
git commit -m "Persist and restore the opaque-background preference"
```

---

### Task 4: "Opaque Background" menu item

**Files:**
- Modify: `Sources/AsciiArcade/main.swift:920-997` (`rebuildMenu`), add a new `@objc` handler near `toggleIdleAutoCycle` (`main.swift:1042-1050`)

**Interfaces:**
- Consumes: `AppDelegate.opaqueBackground` (Task 3), `SceneView.setOpaqueBackgroundEnabled(_:)` (Task 2), `AppDelegate.saveState()` (existing).
- Produces: end-to-end feature — flipping the menu item now visibly changes the desktop backing on every screen and persists across relaunch.

- [ ] **Step 1: Add the menu item in `rebuildMenu`**

In `main.swift:920-997`, add the item next to the other simple on/off toggles — right after the "Launch at Login" item (`main.swift:964-967`) and before the `Capture` separator, so it reads as a general display preference rather than a scene- or DOOM-specific one:

```swift
        let loginItem = NSMenuItem(title: "Launch at Login", action: #selector(toggleLaunchAtLogin(_:)), keyEquivalent: "")
        loginItem.target = self
        loginItem.state = SMAppService.mainApp.status == .enabled ? .on : .off
        menu.addItem(loginItem)

        let opaqueItem = NSMenuItem(title: "Opaque Background", action: #selector(toggleOpaqueBackground(_:)), keyEquivalent: "")
        opaqueItem.target = self
        opaqueItem.state = opaqueBackground ? .on : .off
        menu.addItem(opaqueItem)

        menu.addItem(.separator())
```

(This replaces the existing `menu.addItem(loginItem)` followed directly by `menu.addItem(.separator())` — insert the new item between them.)

- [ ] **Step 2: Add the toggle handler**

Right after `toggleIdleAutoCycle` (`main.swift:1042-1050`), add:

```swift
    @objc func toggleOpaqueBackground(_ sender: NSMenuItem) {
        opaqueBackground.toggle()
        sender.state = opaqueBackground ? .on : .off
        for view in views { view.setOpaqueBackgroundEnabled(opaqueBackground) }
        saveState()
    }
```

- [ ] **Step 3: Build to verify it compiles**

Run: `swift build`
Expected: build succeeds.

- [ ] **Step 4: Manual verification (no automated UI test exists for this file)**

Run: `swift run AsciiArcade`

1. Confirm the desktop background under the ASCII glyphs is solid black for Hacker/Amber/Ice, and unchanged (white) for Ghost.
2. Open the `◎` menu, confirm "Opaque Background" is checked, click it to uncheck — confirm the real desktop wallpaper becomes visible through the glyphs on every screen, for whichever theme is active (including Ghost).
3. Toggle it back on — confirm it goes opaque again immediately, no restart needed.
4. Quit and relaunch — confirm the toggle state you left it in persists.
5. Quit the app (`Cmd+Q` or menu Quit) — confirm no crash and the real desktop wallpaper is exactly as it was before launch (the pre-existing `originalWallpapers` restore path in `applicationWillTerminate`, untouched by this feature, still runs).

- [ ] **Step 5: Commit**

```bash
git add Sources/AsciiArcade/main.swift
git commit -m "Add Opaque Background menu toggle"
```

---

## Self-Review

**Spec coverage:**
- Themes default to opaque black (spec Change 1) → Task 1.
- Menu toggle, on by default, applies to any theme incl. Ghost, persisted, applied live via `applyTheme` path → Task 4 (persistence in Task 3, plumbing in Task 2).
- No `NSWorkspace`/`setWallpaper` touched → confirmed absent from every task; called out explicitly in Global Constraints and Task 4 Step 4.5.
- `originalWallpapers` machinery left untouched → not modified in any task.

**Placeholder scan:** none — every step has literal code or an exact manual-verification checklist.

**Type consistency:** `SceneView.setOpaqueBackgroundEnabled(_ enabled: Bool)` (Task 2) matches its two call sites verbatim — `view.setOpaqueBackgroundEnabled(opaqueBackground)` in Task 3 Step 3 and `view.setOpaqueBackgroundEnabled(opaqueBackground)` in Task 4 Step 2. `PersistedState.opaqueBackground: Bool` (Task 3) matches the single construction call site in `saveState` and the `AppDelegate.opaqueBackground` property it's assigned to/from.
