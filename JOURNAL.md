# JOURNAL — ASCII Arcade

> Dated log of decisions, pivots, incidents, and quotes. Add entries as
> things happen — retrospectives need this raw material to land.
> Reverse-chronological; one paragraph max per entry.

## 2026-08-31: Restored Linux SCTK dependency after CI manifest failure #incident

A config change removed `smithay-client-toolkit` from `shells/linux/Cargo.toml` while the `wayland` feature still referenced it. `cargo fmt` and cargo-deny then failed during manifest parsing, so downstream jobs never ran. Restored the exact `0.19.2` pin and regenerated `Cargo.lock`. On the ARM64 Linux host, `cargo fmt --all --check`, locked metadata, the cargo-deny license check, `cargo check -p aa-linux --all-features --locked`, and targeted clippy all pass. The full workspace test run also found an existing Linux-only `aa autostart` test failure because no autostart service is available on the host. That is separate from the CI regression.

## 2026-08-31: CI covers the xtool package build #decision

CI now builds the SwiftPM iOS package with xtool on macOS and checks that the Metal shader source reaches the app bundle. The existing XcodeGen job remains the simulator smoke test. A Linux ARM64 job runs the full Darwin SDK build when the repository has the Xcode XIP URL and checksum configured as Actions secrets; forked pull requests skip that job because they cannot access those secrets.

## 2026-08-31: xtool packages the Metal shader source #feature #milestone

SwiftPM cannot compile `Shaders.metal` on the Linux xtool path, so the package now copies it into its resource bundle and `GlyphPipelines` compiles it with `MTLDevice.makeLibrary(source:)` when `SWIFT_PACKAGE` is set. The XcodeGen path still calls `makeDefaultLibrary()`. The clean release build passed with exit code 0, and the resulting app contains `AsciiArcade_AsciiArcade.bundle/Shaders.metal` plus a Mach-O arm64 executable. Runtime Metal rendering remains unverified because this host has no iOS device or simulator.

## 2026-08-31: Linux xtool build verified #milestone #incident

The Xcode 26.6 Apple silicon XIP extracted into `darwin.artifactbundle`; registering that bundle requires `swift sdk install`, while `xtool sdk install` expects the original Xcode input. The Linux Rust bridge now uses Swiftly's Clang, xtool's `ld64.lld`, and explicit iOS platform-version flags. The complete release build passed with exit code 0 and produced a non-empty arm64 Mach-O app. SwiftPM still reports `Shaders.metal` as unhandled, so the app's Metal library packaging and runtime rendering need a separate follow-up before this is a functional replacement for the Xcode build.

## 2026-08-31: Linux xtool package added #feature #decision

Kept the existing XcodeGen project as the macOS and IDE path and added a separate SwiftPM package under `shells/ios` for xtool. `scripts/build-ios.sh` now stages only the C headers, keeps Rust intermediates under `AA_IOS_BUILD_ROOT`, and assembles a device-only XCFramework on Linux with the Darwin SDK linker. `scripts/build-xtool-ios.sh` redirects SwiftPM's SDK configuration into the managed SDK tree before building. The package and scripts parse cleanly, and the full `swift test --parallel` suite passes. The end-to-end iOS build remains pending until an Xcode XIP is supplied to generate the Darwin SDK.

## 2026-08-29: Mac scene changes use character dissolves #feature

`SceneView` now keeps the frame it actually painted when a menu or idle-cycle change selects another scene. The new scene renders on its own grid, then takes over cell by cell using deterministic thresholds over 0.9 seconds. Mapping the saved cells to the destination grid keeps the handoff legible across normal text scenes and DOOM's fixed-resolution bitmap path. A second selection during the dissolve starts from the frame on screen, so rapid cycling does not flash or snap. The pure `SceneTransition` helper has core tests; Swift verification remains pending because the designated build host currently has no Swift toolchain.

## 2026-08-23 — Linux Swift coverage uses SwiftPM JSON for deterministic targets #decision

Swift 6.3.3 on ampere-dev emits `llvm.coverage.json.export` rather than an LLVM `.profdata` file on Linux, so the old `xcrun llvm-cov` gate could not run there. Added a small standard-library Python gate for the cross-platform `DonutFrameGenerator` target, which the existing tests exercise at 100% lines and functions. `HotwordDetector` remains tested but is excluded from the line gate because SwiftPM's JSON exporter reports 98.25% lines while reporting 100% functions, counting non-executable declaration/closing lines. macOS-only app, watcher, PTY, and ApplicationServices paths remain explicitly outside the Linux coverage claim; their macOS targets and products stay in the package manifest.

## 2026-07-20 — Scoped out an API plan; no build started #decision

Brainstormed whether ascii-arcade needs a general-purpose API beyond the small `/api/scenes`+ WebSocket surface`shells/web`already has for its own bundled frontend. Conclusion: don't build one speculatively. Wrote`docs/api-plan.md`ranking candidate use cases — a remote-control companion API (phone controls the desktop wallpaper instance instead of duplicating rendering) and a local automation/scripting hook (expose the same verbs the`aa` CLI already has, for Raycast/Stream Deck/cron/Home Assistant triggers) are the strongest fits, since they turn the API into a control surface for the owner rather than inventing a public-service audience. An embeddable widget and streaming-overlay control are plausible but only worth building once there's an actual second consumer; a scene marketplace and multi-instance orchestration are bigger lifts than the current single-user-desktop-tool framing justifies.

## 2026-07-18 — Android live wallpaper shell brought online; aa-ffi's dormant JNI bridge needed a jni 0.22 and AGP 9 rewrite #milestone #incident

Built `shells/android/`end to end and got the donut scene running as a real live wallpaper on an emulator, confirming the same Kotlin-shell/Rust-engine pattern iOS already uses. Turned out`crates/aa-ffi`already had a full Android JNI bridge, left over from the Expo mobile shell that got replaced when iOS went native. It had never been compiled by any CI job and still targeted a Kotlin class that no longer existed. Fixing it surfaced two version-drift problems that weren't obvious just from reading the code.`jni`0.22 split`JNIEnv`into an FFI-safe`EnvUnowned`, which is what a native method actually receives, and a full-featured `Env`that's only reachable inside an`EnvUnowned::with_env`closure; that closure also catches panics automatically, so the panic-across-FFI-boundary handling I was about to write by hand turned out to already be built in. Separately, AGP 9.0 folded Kotlin support directly into the Android Gradle plugin, so the old`org.jetbrains.kotlin.android`plugin is now a hard build error instead of a harmless no-op. Verified past "gradle build succeeded" the same way the iOS and Linux work did before it: installed the APK on an arm64 emulator, screenshotted the preview activity (a recognizable ASCII donut, confirmed animating across two captures), applied it as the actual home-screen wallpaper through the system chooser, then sampled`/proc/pid/stat`CPU ticks to confirm the`WallpaperService.Engine`'s draw loop stops when another app covers it (136 ticks over 5 seconds visible versus 19 ticks over 5 seconds obscured) and starts back up once it's visible again.

## 2026-07-12 — Retired Fire and Clock scenes; DOOM flipped from default to opt-in #decision #pivot

Removed the Fire and Clock scenes from both engines (Swift `AsciiArcadeCore`and the Rust`aa-core`). Clock read as redundant next to the OS's own menu-bar clock, and removing both leaves five scenes. DOOM used to sit in the macOS Scene menu like any other scene; a playable shooter isn't the right default for a machine other people can see (a school computer being the concrete worry), so it's now hidden from the Scene list, ⌘⌥C cycling, and idle auto-cycle until "Enable DOOM Scene" is switched on in the menu (off by default, persisted). Also had to guard the migration: shrinking the scene lineup shifts every index after Fire, so a returning user's saved `sceneIndex`could coincidentally resolve to DOOM's new slot post-update. Added a clamp on launch (and the iOS equivalent for a stale persisted`sceneId`) so that can't bypass the opt-in. Along the way, discovered `crates/aa-doom` was never linked into any of the four Rust shells (Linux/Windows/Web/CLI) at all: a fully built, CI-tested crate with zero consumers. That turned "make DOOM opt-in on the Rust side" from a gating problem into a from-scratch feature decision (see next entry).

## 2026-07-12 — Built DOOM into the Rust CLI and web shells, gated behind a Cargo feature plus a runtime flag #feature #decision

Wired `aa-doom`into`aa play`/`aa web`(and the standalone`aa-web`binary) as a double-gated opt-in: an optional`aa-doom`dependency behind a`doom`Cargo feature (off by default, so a normal build doesn't even link the PTY-spawning GPL code), plus a runtime`--enable-doom` flag (`AA_WEB_ENABLE_DOOM=1`for the standalone binary) that has to be passed on top of that before`aa scenes`/`aa play`/`aa web`will reveal or accept "doom". Hit a real rendering gap doing it:`aa_core::ansi::frame_to_ansi`paints one character per terminal cell with no scaling. That's fine for scenes that resize into whatever grid they're given, but it breaks for DOOM's fixed 640×200 (or scaled) framebuffer. The macOS/wallpaper bitmap compositors only ever scale a small grid up to fill a bigger canvas; a terminal is usually smaller than DOOM's native resolution, so naively printing it just wraps garbage. Fixed it by leaning on`doom_ascii`'s own `-scaling`knob instead of building a downsampler:`aa play doom` now picks the smallest scaling factor whose grid still fits the real terminal size (`terminal::size()`is known before construction), while the web path uses one fixed conservative grid since a browser tab's size isn't known until after the WebSocket handshake. Deliberately left DOOM out of`aa run` (the actual Linux/Windows wallpaper mode): playing DOOM as a literal wallpaper needs fixed-grid bitmap compositing plus global keyboard capture while another app has focus, neither of which the Rust shells implement yet. That's real platform-integration work for its own session, not a gating tweak.

## 2026-07-01 — Live Photo export shipped; PHPhotoLibrary closures must not inherit @MainActor #incident #milestone

Built the Live Photo exporter (offscreen Metal render → AVAssetWriter H.264 + still JPEG, both tagged with a shared `com.apple.quicktime.content.identifier`/maker-note asset ID, saved via `PHAssetCreationRequest`). First on-device test crashed with SIGTRAP on `com.apple.PHPhotoLibrary.changes`—`dispatch_assert_queue_fail`inside`_swift_task_checkIsolatedSwift`. Root cause: `LivePhotoExporter`is`@MainActor`, and `PHPhotoLibrary.shared().performChanges(_:completionHandler:)`'s change-block parameter isn't `@Sendable`-qualified in the Photos overlay, so the closure literal silently inherited `@MainActor`isolation from the enclosing static func.`performChanges`always runs that block on its own private queue, never the main actor — Swift 6's runtime isolation check traps the instant Photos actually invokes it off-main. Fix: mark`requestAddOnlyAuthorization`, `renderAndEncode`, and `saveLivePhoto` `nonisolated static func`. General lesson for this codebase: a closure passed to a plain (non-`@Sendable`) `@escaping`parameter from inside an actor-isolated type inherits that actor's isolation by default — GCD APIs typed with`@Sendable`closures don't have this trap, but many older system APIs (Photos, some AVFoundation completion handlers) aren't updated for that annotation and will silently compile clean, then crash only when the callback actually fires off-main. Verified end-to-end on the iPhone 17 simulator: exported Matrix/Hacker clip landed as a real paired`IMG_0007.MOV`(4.6 MB) +`IMG_0007.JPG` in the camera roll.

## 2026-07-01 — Native iOS renderer was silently drawing nothing; `METAL_LIBRARY_OUTPUT_DIR` was the culprit #incident

The Swift/Metal iOS shell had been building clean since its rewrite, but nobody had actually looked at a screenshot — every prior "it launched" check stopped at the process-alive PID, not the pixels. First real screenshot showed a flat black canvas, no glyphs, HUD fine. `device.makeDefaultLibrary()`was failing at runtime with no crash (Metal just silently has no pipelines), traced via temporary`print`diagnostics in`draw(in:)`to`project.yml`'s `METAL_LIBRARY_OUTPUT_DIR: "$(BUILT_PRODUCTS_DIR)"`— that setting pointed the compiled`default.metallib`at the build products *root*, one level above`AsciiArcade.app/`, so the shader library never made it into the actual app bundle Metal loads from at launch. Removed the override; Xcode's default output path puts it inside the bundle and the Matrix rain rendered immediately after. Same lesson as the 2026-06-24 Wayland theme-color miss: compiling and launching without error is not the same as rendering the right thing — a build-settings typo that Xcode never flags as wrong can still produce a fully broken visual.

## 2026-07-01 — iOS has no live-wallpaper API; pivoted to Live Photo export #decision

Confirmed iOS exposes no public API to set or continuously animate the Home/Lock Screen wallpaper — unlike macOS/Windows/Linux, there's no sandbox door for it, jailbreak-only private APIs aside. The only way ASCII Arcade content reaches an actual iOS wallpaper is the same trick every "video wallpaper" App Store app uses: generate a Live Photo (still JPEG + ~3s H.264 paired via a shared `com.apple.quicktime.content.identifier`+ Apple-maker-note asset ID) and save it via`PHAssetCreationRequest`, then the user manually sets it as a Lock Screen wallpaper. Accepted limitation: iOS only plays the motion on long-press, never continuously — the resting wallpaper is always the still frame. Decided this is worth building anyway since it's the ceiling Apple allows, not a workaround-able gap. Considered and rejected: static image export (throws away the animation entirely, which is the whole point of the app) and WidgetKit (snapshot/timeline-based, heavily throttled by the OS, wouldn't read as a live canvas even with real engineering effort).

## 2026-07-01 — Scrapped Expo/React Native iOS shell; rewrote as fully native Swift #pivot

Replaced `shells/mobile/`(Expo 53 / React Native 0.79 / react-native-skia) with a fully native iOS shell at`shells/ios/`. Stack: SwiftUI app entry, Metal rendering via MTKView, glyph atlas built with CoreText at device pixel density, Rust engine called directly through the aa-ffi XCFramework (same `import AaFfi` module as before). No JavaScript runtime, no Metro bundler, no CocoaPods. XcodeGen (`project.yml`) generates the `.xcodeproj`from source. Target: iOS 27 / Swift 6.0. The Expo shell was functional (Matrix rain working, full-screen background confirmed) but fundamentally wrong for a native-wallpaper-adjacent project — a JS runtime on a platform that already blocks wallpaper access adds nothing. The Metal renderer uses a`GlyphAtlas`(R8Unorm texture, ~300 KB) for ASCII + katakana + block + box-drawing chars, one instanced draw call per frame for all non-space cells; background is a single fullscreen quad.`scripts/build-ios.sh`replaces`build-mobile-ios.sh`and outputs the XCFramework to`shells/ios/Frameworks/`.

## 2026-06-29 — iOS simulator rendering live: ASCII Matrix rain on Canvas #milestone

First working iOS frames. Two root causes had to be peeled in sequence. (1) `SkiaPictureView.js`in`@shopify/react-native-skia@2.6.8`calls`this.tick()`in its constructor — before the Fabric native view is mounted — and`tick()`calls`SkiaViewApi.requestRedraw(nativeId)`where the JSI binding does`.asObject()`on the nativeId number, throwing "Value is a number, expected an Object." Patched: removed the constructor`tick()`call and gated the`componentDidUpdate`tick call to`mode === "continuous"`only. (2) Even with that fixed, using`<SkiaPictureView>`directly still showed black; the correct rendering path for Fabric + Reanimated 3 is`<Canvas><Picture picture={skPicture} /></Canvas>`, which routes through `NativeReanimatedContainer.redraw()`→`Rea.runOnUI()`→`setJsiProperty`on the UI thread, which triggers a Metal surface refresh. (3)`paint.setColor()`requires an`SkColor4f`host object produced by`Skia.Color(input)`, not a raw packed integer — calling it with `(0xff << 24) | ...`threw the same JSI type error. Fixed by using`Skia.Color(hexString)`throughout. Both node_modules patches persisted via`patch-package`; `postinstall`hook added to`package.json`so they survive`npm install`.

## 2026-06-26 — `aa` unified CLI shipped; CI updated to gate it #feature

Added `shells/cli/` crate (`name = "aa"`) that bundles all user-facing surface into one binary: `aa play <scene>`(crossterm terminal renderer, all platforms),`aa run <scene>`(delegates to the native wallpaper shell or opens ASCII Arcade.app on macOS),`aa web <scene>`(embedded axum 0.8 + xterm.js WebSocket server, same transport as`aa-web`), `aa autostart enable/disable/status`(delegates to`aa_linux::autostart`or`aa_windows::autostart`, with a user-facing message on macOS pointing at the status-bar menu), `aa scenes`, `aa themes`. Platform-gated deps in Cargo.toml so the binary compiles everywhere without carrying native linkage it doesn't need. Tokio runtime constructed explicitly — avoids the `#[tokio::main]`overhead for the synchronous subcommands. CI: added`cargo clippy -p aa`to the`core`job (macOS + Windows only — skip Ubuntu because`aa`pulls in`aa-linux`which needs the wayland headers only present in the`shells`job), and added`cargo build/clippy -p aa`to the`shells`job where those headers are already installed.`cargo clippy -p aa`passes`-D warnings` clean.

## 2026-06-26 — Login item / autostart wired on all three platforms #feature

macOS was already done via `SMAppService.mainApp.register()`behind the "Launch at Login" menu item (landed silently in a prior session). Added the Linux and Windows counterparts:`aa_linux::autostart::{install,remove}`writes/removes`~/.config/autostart/ascii-arcade.desktop`(XDG; works on GNOME, KDE, Hyprland, sway);`aa_windows::autostart::{install,remove}`writes/removes`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\AsciiArcade`. Both shells now accept `--autostart-enable [scene] [theme]`and`--autostart-disable`CLI flags. Added`Win32_System_Registry`feature to the`windows`crate dep. Full`cargo check --workspace` passes on macOS.

## 2026-06-26 — E2e demo suite live on Rust web shell; two MP4 recordings produced #milestone

Replaced the Swift Vapor / DOOM webServer in the playwright demo config with the new `aa-web`axum shell. Added`aa_core::ansi::frame_to_ansi`(ANSI truecolor run-length encoder) so any built-in scene can be streamed to an xterm.js terminal over WebSocket. New`shells/web`crate serves a scene-picker page + WebSocket endpoint at`/ws/{scene}`at 30 fps. Demo suite: warmup + DOOM steps generalized to scene-agnostic;`01-doom.feature`repurposed as the donut demo;`02-matrix.feature` exercises the runtime scene switcher. All four tests passed on first run (`npm run demo`) and the reporter wrote `donut-in-the-browser-*.mp4`and`matrix-rain-in-the-browser-*.mp4`to`e2e/recordings/`.

## 2026-06-25 — Windows shell done; icon layer is a Windows Server/RDP limitation #decision

After confirming the wallpaper renders and application windows appear correctly above it, we hit one remaining issue: desktop icons disappear when aa-windows runs on Windows Server 2019 via RDP. Tried five approaches (WS_CHILD + HWND_BOTTOM, WS_POPUP + SetWindowPos, dropping WM_SPAWN_WORKERW) — none restored the icon layer on Server. Root cause: Windows Server's desktop shell in an RDP session uses a different DWM composition path from Windows 10/11 consumer editions; the icon layer (SHELLDLL_DefView) doesn't coexist with a custom GDI wallpaper surface the same way. Confirmed NOT a code bug: normal application windows (Server Manager, CMD) correctly appear above the wallpaper in all tests. Declared the Windows shell done pending one native Windows 10/11 desktop test for icon behavior. GCP VM stopped; branch merged to main.

## 2026-06-25 — WorkerW confirmed working on real Windows (GCP VM via FreeRDP) #milestone

Ran `aa-windows.exe donut`on a GCP Windows Server 2019 spot VM (e2-medium, us-central1-a) connected via FreeRDP. The WorkerW technique works: the ASCII donut rendered behind the CMD window and other normal windows, confirming the wallpaper layer is correctly below the interactive window stack. Three bugs hit and fixed before the render succeeded: (1) missing VCRUNTIME140.dll — fixed permanently with`target-feature=+crt-static`in`.cargo/config.toml`; (2) no app icon — fixed by generating `icon.ico`from`assets/logo.svg`via rsvg-convert + ImageMagick and embedding via`winres`in`build.rs`; (3) `WS_CHILD`+ null parent crash —`CreateWindowExW`with`WS_CHILD`style requires a valid parent HWND; passing`HWND::default()`crashed silently; fixed by passing the WorkerW host HWND directly. One known caveat specific to Windows Server / RDP sessions: the desktop icons disappear after`WM_SPAWN_WORKERW`— on real Windows 10/11 consumer desktops the icon layer survives. Also patched:`#![windows_subsystem = "windows"]` added so the binary runs without a console window and can't be killed by closing one.

## 2026-06-24 — Saw the actual pixels: headless sway + grim, caught a theme bug #milestone #incident

Closed the "never eyeballed the render" gap on the Wayland side without a physical display: ran **headless sway**on `ampere-dev` (`WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman`), had its config `exec`launch`aa-linux`+`grim`to screenshot the output, then`scp`'d the PNG back to look at it. First capture (`aa-linux matrix amber`) showed the Matrix rain rendering correctly as a real layer-shell background surface — but**green, not amber**. The bug: the Windows/Linux shells never called `Scene::apply_base_color(theme.text)`, so colour scenes ignored the theme (the macOS host does call it — that's why Matrix "turns amber under Amber" there). Every compile/clippy/test gate was green; only looking at the pixels caught it. One-line fix in each shell; re-captured and confirmed amber. Lesson reinforced (cf. the DOOM-upside-down entry): pick a concrete visual expectation and verify against it — "it compiles and renders something" isn't "it renders the right thing."

## 2026-06-24 — Wayland backend written + whole stack verified on real Linux #milestone

Owner handed over an Ubuntu 24.04 / aarch64 VM (`ampere-dev`, Oracle Cloud) — the Linux build host the Wayland backend needed. Workflow: author the `wlr-layer-shell`backend locally with the normal edit tools,`rsync`the workspace to the VM, and`cargo check`/`clippy`/`test`there natively (the VM is headless — no compositor — so this verifies compilation and the full test suite, not on-screen rendering). The Wayland module is a real smithay-client-toolkit`background`-layer surface (handlers + delegate macros + `SlotPool`shm +`wl_surface.frame`-driven animation); first compile surfaced only 8 `u32`/`usize`mix-ups in the blit loop — the SCTK trait/delegate wiring was right. After that:`aa-linux`(X11 **and**Wayland) compiles +`clippy -D warnings`clean, and the**entire workspace's tests pass natively on aarch64**(39+28+9). Bonus wins from having a real Linux box: caught that`scripts/setup.sh` was zsh-only (`CLEANUP() { … }`missing its`;`, plus a `#!/bin/zsh`shebang) — fixed to`#!/usr/bin/env bash`, then it built `doom_ascii` on Linux and the**DOOM e2e spawn test passed over forkpty**, so DOOM-on-Rust is now proven on macOS *and* Linux. Residual gap is now narrow and honest: the shells compile + run (graceful headless failure confirmed) but on-screen wallpaper rendering — WorkerW on real Windows, layer-shell on a real wlroots/KDE display — still hasn't been eyeballed.

## 2026-06-24 — Both native shells landed (WorkerW + X11) #milestone

After the agent assigned the shells got only as far as the Linux Cargo features before the session cutoff, finished both by hand. `aa-windows`: the WorkerW dance (spawn via Progman `0x052C`, `EnumWindows`for the`SHELLDLL_DefView`sibling,`SetParent`a render window in) + a per-frame GDI`StretchDIBits`blit of the`aa_render`buffer (BGRA, top-down DIB).`aa-linux`: an X11 root-pixmap backend on pure-Rust `x11rb`— paint a pixmap, publish`_XROOTPMAP_ID`/`ESETROOT_PMAP_ID`, re-blit + `clear_area`each frame,`put_image` banded under the server's max request size; also covers XWayland. Both verified by cross-`cargo check`+`clippy -D warnings` (`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`) — they can't link on the macOS dev box, so CI's native runners are the runtime gate. The honest gap: the **Wayland `wlr-layer-shell` backend is a documented stub** — its crates (`smithay-client-toolkit`/`wayland-client`) link `libwayland` and won't even compile-check from macOS, so writing it blind would ship unverifiable code; it needs a Linux build host or a CI iteration loop. X11/XWayland covers the verifiable path for now.

## 2026-06-24 — Rust engine ported + DOOM proven over portable-pty #milestone

Fanned the Rust port out across four parallel background agents (worktree-isolated, one crate each): scenes, rasteriser, DOOM driver, shells. Three landed cleanly; all four hit a session limit mid-task and none committed, so the orchestrator captured their uncommitted worktrees, merged, and finished the wiring. Result on `rust-crossplatform`: `aa-core`has all seven scenes (donut/helix/matrix/fire/pipes/life/clock) + the`Stepper`fixed-timestep helper;`aa-render`rasterises frames through an embedded 8×16 font with glow + scanline FX;`aa-doom`drives real`doom_ascii`over`portable-pty`— the **end-to-end spawn is verified rendering on macOS**, which is the proof the cross-platform DOOM bet holds (same API = ConPTY on Windows, forkpty on Linux). Workspace is green: tests +`clippy -D warnings`+`fmt`. Two real bugs caught integrating agent output: a float-drift accumulator in `Stepper` that silently swallowed a due step (fixed with an epsilon), and a screen-buffer test that wrongly assumed terminal auto-wrap (doom emits an explicit newline per scanline, so no-wrap is correct). **Still open:** the two native shells (`aa-windows`WorkerW,`aa-linux` X11/layer-shell) are still skeletons — the agent assigned them only got as far as the Linux Cargo features before the cutoff. That's the actual product layer and the next real build.

## 2026-06-24 — Going cross-platform native: Rust engine + per-OS wallpaper shells #decision #pivot

Owner wants the *native* wallpaper experience (a window behind the desktop icons) on Windows and Linux, not just the existing browser path. Accepted the irreducible cost: a native wallpaper is a different OS mechanism per platform (macOS desktop-level `NSWindow`; Windows WorkerW reparenting à la Lively; Linux X11 root-pixmap + `wlr-layer-shell`), so the shells can't share code — only the engine can. Decision: **rewrite the ~1800-line `AsciiArcadeCore` engine in Rust** rather than ship the Swift core as a C-ABI lib or go Swift-everywhere. Rationale — Rust has the best-supported shell tooling on every target (`windows`crate for WorkerW,`smithay`/layer-shell + `x11rb`for Linux), and`portable-pty`covers ConPTY *and* forkpty uniformly, which solves DOOM-on-Windows for free (current`PTYBridge`is`forkpty`-only, doesn't exist on Windows). Linux scope deliberately bounded to X11 + wlroots/KDE-Wayland; **GNOME-Wayland punted** (needs a Shell extension — biggest pain, separate distribution burden). Open question deferred: whether the macOS shell stays Swift/AppKit (keep working code) or also moves to Rust for a single codebase.

## 2026-06-24 — Stable signing identity to stop repeated Accessibility prompts #incident #decision

`make-app.sh` ad-hoc signed (`codesign --sign -`), which pins the app's designated requirement to the binary's `cdhash`. Every rebuild changed the hash, so macOS TCC treated each reinstall as a brand-new app and re-prompted for Accessibility, leaving a graveyard of dead grants. Fix: a one-time self-signed code-signing identity (`scripts/setup-signing.sh`) makes the requirement identity-based (`identifier … and certificate leaf = H"…"`) and constant across builds. Gotchas hit along the way: OpenSSL 3 writes a PKCS#12 MAC that macOS `security import`rejects ("MAC verification failed") — needs`-legacy -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1`plus a non-empty password; and the self-signed cert reports`NOT_TRUSTED`so the`make-app.sh`guard must list identities without`-v`(valid-only hides it). codesign signs with the untrusted cert fine, and TCC pins the leaf hash literally so trust is irrelevant.`reinstall.sh` now self-heals by recreating the identity if it's missing.

## 2026-06-24 — DOOM gamma defaulted to level 2 (was OFF) #decision

Owner asked "is there supposed to be this much shadow?" looking at a near-black DOOM frame. It was faithful — DOOM ships with `usegamma`OFF (its darkest), so unlit/distant sectors fade to pure black, and we paint that honestly (confirmed it's opaque black colour data, not the transparent→white-matte capture artifact, so not a missing-pixel bug). But "authentic" isn't the goal for a wallpaper. The bundled`doom_ascii`accepts`-fixgamma N`(0=off … 4=brightest; same as the in-game F11 /`key_menu_gamma`). Added a `DOOM_GAMMA`env knob to`DoomLauncher`, default 2 — dev capture went from a black void with two lit pillars to a fully legible room (walls, fences, enemies, stairs). Fits the owner's standing preference for legible-bold visuals over noise.

## 2026-06-24 — DOOM rendered upside down since the bitmap path landed #incident

`drawBitmap`computed each cell's y as`rect.maxY - (row+1)*cellH`, but `SceneView.isFlipped`is`true`(y grows downward, origin top-left) — so row 0 landed at the bottom and the whole framebuffer rendered vertically flipped. The glyph path dodges this with its`translateBy(y:viewH)/scaleBy(1,-1)`transform; the bitmap path drew directly and never compensated. Fix: map row 0 to`rect.minY` (`yTop = rect.minY + row*cellH`). Lesson worth keeping: this shipped flipped through *four* of my own verification screenshots and I read them all as fine — an HUD-at-top DOOM frame looks plausible enough to fool a glance, and I anchored on "is it colourful and detailed" instead of "is the status bar where it belongs." The owner caught it in one look. Pick a known landmark (DOOM status bar = bottom; messages = top) before calling a frame correct.

## 2026-06-24 — DOOM default resolution bumped to native 320×200 #decision

After the black-screen fix, owner asked "is this the clearest DOOM can be in ASCII?" — it wasn't; the default was `scaling=2`(160×100). Demoed`scaling=1`side by side: at native 320×200 the HUD digits, the red menu text, the blue "NEW GAME", and the FREEDOOM ∞ wordmark all become legible where they were mush before. doom_ascii can't exceed this — it's the engine's internal framebuffer. Owner chose native-as-default over a menu toggle or keeping the lighter mode, accepting the cost (~24-28fps + ~40ms/frame vs ~30fps + ~16ms). Changed`DoomScene`'s default `scaling`2→1;`DOOM_SCALING`env still overrides for a lighter frame. Worth remembering: for DOOM the app doesn't render glyphs at all —`drawBitmap` paints each cell as a solid colour rect, so "clarity" is purely pixel resolution; true ASCII-ramp glyphs would read more retro but less clear.

## 2026-06-24 — DOOM black-screen regression: stripped binary + invisible message #incident

Reported "DOOM now just shows a black screen." Two compounding causes: (1) the recent reinstalls defaulted to `INCLUDE_DOOM=0`, so the installed `.app`no longer bundled`doom_ascii`and DOOM hit the "not found" message path; (2) the new fixed-resolution`drawBitmap`host path (added for the pixelation fix) skipped every cell with a`nil`colour — and`showMessage`writes uncoloured text — so the message painted nothing and read as pure black. Fixed`drawBitmap`to fall back to`themeTextColor`for uncoloured non-blank cells (matching the glyph path), and reinstalled with`INCLUDE_DOOM=1`. Verified the render path itself was never broken: a dev build run from the repo root (where `bin/doom_ascii` resolves) shows DOOM crisp at grid 320×100, ~30fps — HUD, face, and wall textures all sharp, exactly the resolution win the pixelation rework was after.

## 2026-06-23 — Added in-app screenshot and 3-second clip recorder #milestone #decision

Made capturing the wallpaper a first-class feature after observing that macOS's native ⌘⇧3/4 skips the desktop-level window entirely (it samples the wallpaper compositor, not the live window backing store). The fix lives in the app: "Save Screenshot (⌘⌥S)" and "Record 3-Sec Clip (⌘⌥R)" under a new Capture section in the menu bar. Both use `CGWindowListCreateImage(.null, .optionIncludingWindow, windowID, .bestResolution)`to pull directly from the window's backing store — this bypasses the compositor and works regardless of window level. Screenshot saves a PNG to ~/Desktop and also copies it to the clipboard (so ⌘V works immediately). The clip recorder fires a`DispatchSourceTimer`at 15 fps on a background serial queue, converts each`CGImage`to a`CVPixelBuffer`, feeds it to an `AVAssetWriterInputPixelBufferAdaptor`, then finalises a .mp4 via `AVAssetWriter`at the 3-second mark (or earlier on manual stop). The status-bar`◎`button blinks`◉`during recording and flashes`✓`/`✗` on outcome — the user never sees "window level", "compositor", or "backing store". Design call: both shortcuts (⌘⌥S, ⌘⌥R) are handled in the global NSEvent monitor alongside the existing ⌘⌥C scene-cycle shortcut, so they work even when no app is frontmost.

## 2026-06-23 — Made it a real desktop app (unsigned, self-distributed) #decision #milestone

Promoted ascii-arcade from a `swift run`tool to an installable`.app` + DMG.
Owner's call on scope: do Tier 1 (proper local app) *and* Tier 2 (distribution)
but **skip the Apple Developer account** — recipients bypass Gatekeeper manually
(right-click → Open / `xattr -dr com.apple.quarantine`). Added: `UserDefaults`
persistence of scene/theme/capture/idle/per-scene settings (restored on launch;
returning users also get their theme wallpaper back, first-run leaves the desktop
alone); launch-at-login via `SMAppService.mainApp`; bundle-aware resource lookup
in `DoomLauncher`(checks`Bundle.main` so WADs/doom resolve from the .app, not
just cwd); and `scripts/make-app.sh`+`make-dmg.sh` (release build → Info.plist
with `LSUIElement`→`.icns` from the 512 PNG → bundled Freedoom WADs → ad-hoc
sign → DMG with an Applications drop + first-launch note). Verified the full
save/quit/relaunch/restore cycle against the bundled app. Honoured the existing
"don't redistribute GPL doom_ascii" policy: WADs (BSD) are bundled, doom_ascii is
behind an opt-in `INCLUDE_DOOM=1` flag. Mac App Store stays off the table —
global key capture + arbitrary wallpaper-setting don't survive sandboxing.

## 2026-06-23 — Life read as noise; reseeded it with classic patterns #feedback #decision

Owner watched the Life scene and said the designs weren't clear — it looked like
a sparse scatter rather than anything recognisable. Root cause: I'd seeded it
with uniform random soup, which Conway's rules famously decay into "ash" (a
field of tiny 1–2 cell still-lifes and blinkers). Three changes: (1) seed with
*curated* patterns — gliders, lightweight spaceships, pulsars, Gosper glider
guns, acorns/R-pentominoes — stamped at random positions/orientations, so it
grows recognisable shapes and sustained motion; (2) run the sim on a coarser
*logical* grid scaled up into solid `█` blocks (new Cell-size setting, default
3×3 px/cell) so structures are big enough to read; (3) drop the now-irrelevant
random-density setting. The `#` glyph at 1-px cells was a big part of the "looks
like noise" problem — solid blocks read far better. Also a reminder logged: the
desktop window is transparent, so `screencapture` composites it over a white
matte instead of the real black desktop — the grey background in shared
screenshots is a capture artifact, not the actual wallpaper.

## 2026-06-23 — Fixed the colour-scene lag with a batched Core Text renderer #incident #decision

The per-cell colour path I'd just added lagged badly on the dense scenes (Fire,
Matrix). Root cause: `draw(_:)`rebuilt an`NSMutableAttributedString` every
frame and called `addAttribute(.foregroundColor, range:)` per colour run — and
on a smooth gradient like Fire almost every one of the ~10k cells is its own
run, so that's thousands of attribute mutations plus a full Cocoa text layout
every frame, at the display's full refresh rate, re-measuring the font each
frame too. Replaced the whole text path with a **batched Core Text renderer**:
bucket every non-blank cell's glyph by colour, then one `setFillColor` +
`CTFontDrawGlyphs` per bucket. Fire's ~10k cells collapse to ~35 buckets (its
37-entry palette), Matrix to ~230, Donut to 1. Also cached font metrics +
glyph + CGColor lookups (no per-frame `("@").size(...)`), and capped redraws to
~30fps via the `CVDisplayLink` callback (ASCII doesn't need 120Hz; the text
fill is the hot path). Measured after: avgDraw **~8ms/frame** across Donut/
Matrix/Fire at a 176×57 grid — comfortably inside the 33ms budget, steady
23–25fps with no stutter (was visibly lagging before). Gotcha: drawing Core
Text glyphs upright in an `isFlipped` `NSView` needs an explicit
`translateBy(y: height) + scaleBy(y: -1)` on the context with positions
converted to that y-up space. Instrumentation left behind, env-gated: run with
`ASCII_FPS=1` to log scene/grid/fps/avgDraw/batches once a second.

## 2026-06-23 — Expanded the cabinet: five scenes + per-cell colour #milestone #decision

Added five new scenes (Matrix rain, Doom-fire, Conway's Game of Life, a pipes
screensaver, and a big block-digit clock) and a colour pipeline so they're not
all stuck in one theme tint. The design call that made this clean: a
platform-neutral `RGBColor`/`ColoredFrame`in`AsciiArcadeCore` (no AppKit) plus
an optional `coloredFrame(atTime:)`on`AsciiScene`that defaults to`nil` — so
the donut/helix monochrome path is untouched and the host only takes the
per-cell-colour branch when a scene opts in. The four stateful scenes share a new
`SteppedScene`base that converts the host's "frame at time`t`" pull into a
fixed-timestep simulation (accumulate `dt`, clamp after stalls, cap catch-up
steps) — everything runs on the main thread, so unlike `DoomScene` it needs no
locking. DOOM now keeps the SGR truecolor it used to discard (`DoomScreenBuffer`
tracks a `currentColor` and a parallel colour grid), so it renders in its native
palette on the desktop too. Gotcha worth remembering: in the AppKit host
`RGBColor` is ambiguous because AppKit transitively imports the legacy Quickdraw
`RGBColor` from ApplicationServices — had to qualify it as
`AsciiArcadeCore.RGBColor`. Also added *Scene Settings* (per-scene discrete knobs
surfaced as menu submenus) and *Auto-cycle when idle* (poll `CGEventSource`
idle seconds; slideshow the scenes after 90 s, snap back on input; pause
rendering on display sleep via `NSWorkspace` notifications).

## 2026-06-10 — Pushed public + CI toolchain mismatch #milestone #incident #decision

Pushed to [github.com/Builder106/ascii-arcade](https://github.com/Builder106/ascii-arcade) (public; description + 11
topics) and added a 1200×630 social-preview card. The first CI run failed on a
real toolchain mismatch: the macos-14 runner ships Swift 5.10, which can't read
the `Package.resolved` (format v3) my local Swift 6.3 wrote — so it discarded the
pin and re-resolved to the latest Vapor (4.121.4), which itself requires Swift
tools 6.0 → `error: using Swift tools version 6.0.0 but the installed version is
5.10.0`. Fixed by moving CI to `macos-15` (Xcode 16 / Swift 6) to match the
committed pin, and bumped `actions/checkout` v4→v5 to clear the Node 20
deprecation. Takeaway: **this project now requires a Swift 6 toolchain** because
the committed Vapor pin (4.121.4) declares tools 6.0 — worth stating in the
README's build requirements if older toolchains need support.

## 2026-06-10 — Scaffolded the repo baseline #milestone #incident

Added the storefront baseline: hand-authored SVG banner (light/dark, 1200×420)
with PNG fallbacks, a phosphor-donut logo + apple-touch-icon, shields.io badges,
a macOS CI workflow (build+test, plus a job proving setup.sh still builds
doom_ascii), and a playwright-bdd demo suite for the browser-DOOM surface. Two
environment gotchas surfaced while validating the demo: SwiftPM's package cache
breaks under a global git `safe.bareRepository=explicit` (worked around with a
`GIT_CONFIG_*`env override), and SwiftPM's`build.db`throws`disk I/O error`
on this Google-Drive-synced checkout — recording the live demo needs a
local-disk clone. The scaffold is validated (`bddgen` generates the specs); the
live capture is left to the user on local disk.

## 2026-06-10 — Merged donut + DOOM into ascii-arcade #milestone #decision

Combined the two sibling projects into one repo with `git subtree` so both
commit histories are preserved. The unifying idea (per the owner): not two
separate things, but one live-wallpaper customizer where the spinning donut and
playable text-mode DOOM are both selectable desktop backgrounds. DOOM became
just another `AsciiScene` rendered with the same CRT text drawing as the donut.

## 2026-06-10 — DOOM-as-wallpaper needs a screen buffer, not a terminal #decision

`doom_ascii` emits each frame as a full ANSI redraw — cursor-home (`ESC[;H`),
optional clear, then per-pixel truecolor SGR codes followed by a block glyph.
Rather than embed a full terminal emulator, wrote a minimal `DoomScreenBuffer`
that honors home/clear/erase and strips the SGR colour codes — just enough to
reconstruct the glyph grid for a monochrome themed wallpaper. The block glyphs
happen to suit the donut aesthetic.

## 2026-06-10 — Kept the Vapor browser path as a bonus #decision

The product is now desktop-first, but chose to keep `Server`/`Hotword` /
`WatcherCLI` so DOOM stays playable in a browser tab too (useful where global
keystroke capture isn't available). Refactored the server's binary/IWAD lookup
into a shared `DoomLauncher` so the app and the server resolve DOOM identically.

## 2026-04-29 — Donut wallpaper host landed #milestone

The `donut`project's initial commit:`DonutCore` (the torus and helix frame
generators) plus an AppKit host that paints ASCII into a desktop-level window
with CRT scanlines, a soft glow, and theme presets (Hacker / Amber / Ice /
Ghost). This host is the foundation ASCII Arcade is built on.

## 2026-02-19 — Ghost Protocol audit stub #incident

A `GOALS.md` was auto-generated for the DOOM project by a "Ghost Protocol audit"
showing 0/0 goals complete — a sign the project had drifted with no tracked
objectives. Dropped during the merge in favor of this journal.

## 2025-09-24 — DOOM-over-PTY prototype #milestone

DOOM's initial SwiftPM workspace: a PTY bridge wrapping `doom_ascii`, a Vapor
WebSocket server streaming frames to an xterm.js frontend, a KMP-style hotword
detector, and a LaunchAgent watcher that opened the browser on the hotword. Its
README summed up the state: "Working in pieces — end-to-end integration is the
rough edge." The merge turned that PTY bridge into the heart of the DOOM scene.
