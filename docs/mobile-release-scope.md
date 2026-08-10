# Scope: Android and iOS in the release pipeline

`release.yml` currently packages three platforms on a `v*` tag push —
macOS (universal DMG), Windows (`.exe`), Linux (`.tar.gz`) — via
`release-macos` / `release-windows` / `release-linux` jobs feeding one
`publish` job. `shells/android/` and `shells/ios/` are real, buildable
projects (Gradle 9.2 + Kotlin, and an XcodeGen-generated Xcode project
targeting iOS 26) with their own `scripts/build-android.sh` and
`scripts/build-ios.sh`, but neither has a release job. This doc scopes
adding them.

## Android

**What exists:** `shells/android/` builds against the Rust core through
`scripts/build-android.sh`. No release job.

**What's missing to ship a downloadable APK/AAB:**

- **Signing.** An unsigned release build installs nowhere outside a
  developer's own device. Needs a release keystore, and the keystore
  password + key alias + key password stored as repo secrets
  (`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, etc.), decoded
  in CI before `gradlew assembleRelease` / `bundleRelease`.
- **Distribution format decision.** A sideloadable APK (parallels the
  DMG's "download and run" model, no store review) vs. an AAB for Play
  Store distribution (store review, staged rollout, but auto-updates and
  Play Protect trust). These aren't mutually exclusive — could ship both
  — but the site's Download button and the install-section copy need to
  know which one they're pointing at.
- **If Play Store:** a developer account ($25 one-time), a store listing
  (description, screenshots, privacy policy URL — the site's "No
  account, no telemetry" claim will need a real privacy policy page to
  back it up in the listing), and content rating questionnaire.
- **If sideloaded APK only:** same unversioned-filename trick as the DMG
  (`/releases/latest/download/ASCII-Arcade.apk`) to keep the direct-link
  pattern working, plus install copy explaining "install from unknown
  sources," which is a rougher UX than Gatekeeper's control-click.
- **CI runner:** `ubuntu-latest` is fine for a Gradle build — no macOS
  runner cost here, unlike iOS.

## iOS

**What exists:** `shells/ios/` is a full XcodeGen project
(`project.yml`) targeting iOS 26, linking an `AaEngine.xcframework` that
`scripts/build-ios.sh` builds from the Rust core if missing.
`DEVELOPMENT_TEAM` in `project.yml` is currently blank. No release job.

**What's missing to ship a downloadable build:**

- **Apple Developer Program membership** ($99/year) — required for any
  distribution method beyond a 7-day free-provisioning sideload.
- **Signing identity + provisioning profile**, and a real
  `DEVELOPMENT_TEAM` value. In CI this means either:
  - `fastlane match` (or manual) syncing a distribution certificate +
    profile into the runner's keychain, secrets-gated, or
  - App Store Connect API key auth for `xcodebuild -exportArchive` /
    `altool`/`notarytool` uploads.
- **Distribution format decision**, and this one is more constrained
  than Android's:
  - **App Store**: the only path to an install experience remotely like
    the DMG's "download and open." Requires App Store review (days, not
    minutes — breaks the "download it and it just works" promise the
    site currently makes for macOS), a listing, screenshots, and app
    review guideline compliance (a wallpaper/background-rendering app is
    a use case Apple's reviewers may push back on if it does anything
    resembling a live wallpaper without a supported iOS mechanism for
    that).
  - **TestFlight**: no public direct-download link at all — testers must
    be invited by email/link and install the TestFlight app first. Not
    a fit for the site's current "click Download, get the app" flow.
  - **Ad-hoc / notarized direct distribution**: technically possible
    (`.ipa` + manifest plist + `itms-services://` link) but requires
    UDID registration per device before the OS will trust it, which
    doesn't work for an anonymous website visitor at all.
  - **Practical implication:** unlike Android's sideload option, there
    is no iOS equivalent to "click Download, get a working app" for an
    unknown visitor. App Store is very likely the only viable choice if
    the goal is public distribution, which means accepting review
    latency and guideline risk as part of this scope, not just CI work.
- **CI runner:** needs `macos-*` (Xcode toolchain), same tier as the
  existing `release-macos` job.

## Cross-cutting

- **Whether this app conceptually fits "wallpaper" on mobile at all.**
  macOS's pitch is "draws behind your windows." Neither Android nor iOS
  exposes a live-wallpaper API to third-party apps the way Android's own
  first-party `WallpaperService` does for Android specifically (iOS has
  no live wallpaper API for third-party apps, full stop — only static
  images via Shortcuts workarounds). Confirm what these mobile shells
  actually *are* — a live wallpaper (Android only, via
  `WallpaperService`), a standalone viewer app, or something else —
  before scoping the release mechanics further, since it changes both
  the site copy and the store-listing category.
- **Site changes**, once the above is settled: new Download buttons or
  a platform picker (the current single DMG-shaped button doesn't
  generalize to "here are four different install flows"), updated
  install-section copy per platform, and updated meta description /
  OG tags (currently macOS-only: "A macOS live wallpaper...").
- **release.yml changes**, mechanical once signing is sorted: new
  `release-android` / `release-ios` jobs parallel to the existing three,
  feeding the same `publish` job's `dist-release/*` glob upload.

## Suggested order

1. Settle what the Android and iOS shells actually deliver (live
   wallpaper vs. viewer app) — this changes everything downstream.
2. Android first: cheaper (no paid account required for sideloading,
   no review latency), and the CI mechanics are simpler.
3. iOS: budget for the Developer Program fee, review lead time, and
   accept that "instant public download" isn't achievable the way it is
   for macOS/Windows/Linux/Android-sideload — App Store is the realistic
   target.
