import { defineConfig, devices } from "@playwright/test";

// The marketing site suite. Separate from playwright.demo.config.ts, which
// records the aa-web narrative demos and is always passed with --config; this
// one is the default so `npx playwright test` runs the site.
//
// The server is a plain static file server over site/, because the site has
// no build step of its own: site/pkg/ and site/doom-wasm/ are produced ahead
// of time by scripts/build-wasm.sh and scripts/build-doom-wasm.sh.
export default defineConfig({
  testDir: "tests",
  fullyParallel: true,
  reporter: [["list"]],
  // Serves site/ at the root, as Vercel does: 404.html uses root-absolute
  // paths so it works at any depth. Bound to loopback deliberately:
  // http.server defaults to 0.0.0.0, which on a VM with a public address
  // would expose the directory for the length of a test run.
  webServer: {
    command: "python3 -m http.server 8899 --bind 127.0.0.1 --directory ../site",
    port: 8899,
    reuseExistingServer: true,
    stdout: "ignore",
  },
  use: {
    baseURL: "http://127.0.0.1:8899",
    ...devices["Desktop Chrome"],
  },
  // The cabinet specs render the 3D cabinet in real time, which on a
  // machine without a GPU is slow enough that sharing it with another
  // worker makes their timing flaky. They run after everything else, so
  // nothing else competes with them.
  projects: [
    { name: "site", testIgnore: /cabinet\.spec\.ts/ },
    { name: "cabinet", testMatch: /cabinet\.spec\.ts/, dependencies: ["site"], fullyParallel: false },
  ],
});
