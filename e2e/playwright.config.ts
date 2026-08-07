import { defineConfig, devices } from "@playwright/test";

// The marketing site suite. Separate from playwright.demo.config.ts, which
// records the aa-web narrative demos and is always passed with --config; this
// one is the default so `npx playwright test` runs the site.
//
// The server is a plain static file server over the repo root, because the
// site has no build step of its own: site/pkg/ is produced ahead of time by
// scripts/build-wasm.sh on the VM.
export default defineConfig({
  testDir: "tests",
  fullyParallel: true,
  reporter: [["list"]],
  webServer: {
    command: "python3 -m http.server 8899 --directory ..",
    port: 8899,
    reuseExistingServer: true,
  },
  use: {
    baseURL: "http://127.0.0.1:8899",
    ...devices["Desktop Chrome"],
  },
});
