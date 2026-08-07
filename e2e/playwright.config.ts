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
  // Serves the repo root because the page resolves ../assets/* as /assets/*.
  // Bound to loopback deliberately: http.server defaults to 0.0.0.0, which on
  // a VM with a public address would expose the whole working tree, .git and
  // any untracked files included, for the length of a test run.
  webServer: {
    command: "python3 -m http.server 8899 --bind 127.0.0.1 --directory ..",
    port: 8899,
    reuseExistingServer: true,
    stdout: "ignore",
  },
  use: {
    baseURL: "http://127.0.0.1:8899",
    ...devices["Desktop Chrome"],
  },
});
