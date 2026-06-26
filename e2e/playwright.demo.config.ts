import { defineConfig, devices } from "@playwright/test";
import { defineBddConfig } from "playwright-bdd";

// Narrative demo recordings of the browser-facing web shell. The wallpaper
// apps (Windows/Linux) are desktop overlays recorded manually; this suite
// covers the headlessly-recordable surface: the Rust aa-web server streaming
// scenes as ANSI truecolor to an xterm.js terminal in the browser.
const testDir = defineBddConfig({
  features: "demo/features/**/*.feature",
  steps: "demo/steps/**/*.ts",
});

const PORT = process.env.AA_WEB_PORT ?? "8788";

export default defineConfig({
  testDir,
  timeout: 180_000,
  fullyParallel: false, // see "0-byte first-test video bug" in CLAUDE.md
  workers: 1,
  retries: 0,
  reporter: [["list"], ["./demo/reporters/video-reporter.ts"]],
  // Build + boot the Rust web shell, then run the demo against it. `cargo build`
  // is cached after the first run so subsequent demo runs start in a few seconds.
  webServer: {
    command:
      `cargo build -p aa-web --manifest-path ../Cargo.toml && ` +
      `AA_WEB_PORT=${PORT} ../target/debug/aa-web`,
    url: `http://127.0.0.1:${PORT}`,
    timeout: 300_000,
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
