import { defineConfig, devices } from "@playwright/test";
import { defineBddConfig } from "playwright-bdd";

// Narrative demo recordings of the browser-DOOM bonus surface. The wallpaper
// app itself is a desktop overlay and is recorded manually; this suite covers
// the one headlessly-recordable surface (the Vapor server + xterm.js page).
const testDir = defineBddConfig({
  features: "demo/features/**/*.feature",
  steps: "demo/steps/**/*.ts",
});

const PORT = process.env.DOOM_PORT ?? "8787";

export default defineConfig({
  testDir,
  timeout: 180_000,
  fullyParallel: false, // see "0-byte first-test video bug" in CLAUDE.md
  workers: 1,
  retries: 0,
  reporter: [["list"], ["./demo/reporters/video-reporter.ts"]],
  // Build + boot the Vapor server, then run the demo against it. The server runs
  // with cwd = this e2e dir, so pin the binary/WAD paths explicitly to the repo root.
  webServer: {
    command:
      // GIT_CONFIG_* override: some setups (incl. this repo's owner) set
      // git's `safe.bareRepository=explicit` globally, which breaks SwiftPM's
      // package cache. Injecting it here keeps `swift run` from re-fetching.
      "export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.bareRepository GIT_CONFIG_VALUE_0=all; " +
      "../scripts/setup.sh >/dev/null 2>&1 || true; " +
      "swift build --package-path .. && " +
      `DOOM_PORT=${PORT} ` +
      'DOOM_ASCII_PATH="$PWD/../bin/doom_ascii" ' +
      'DOOM_WAD_DIR="$PWD/../wad" ' +
      "swift run --package-path .. Server",
    url: `http://127.0.0.1:${PORT}`,
    timeout: 240_000,
    reuseExistingServer: true,
  },
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    headless: true,
    viewport: { width: 2560, height: 1600 },
    video: { mode: "on", size: { width: 2560, height: 1600 } },
    launchOptions: {
      slowMo: Number(process.env.DEMO_SLOWMO ?? 1000),
    },
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // Re-pin: the device preset silently overrides the top-level use block.
        viewport: { width: 2560, height: 1600 },
        video: { mode: "on", size: { width: 2560, height: 1600 } },
      },
    },
  ],
});
