import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { brotliCompressSync } from "node:zlib";
import { test, expect, built, SITE_DIR } from "./fixtures";

// docs/landing-page-design.md: HTML, CSS, JavaScript, fonts and the aa-core
// WebAssembly stay under 150 kB. Everything that loads after the page is up
// sits outside that: DOOM's recording, its playable build (on request only),
// and the cabinet with its three.js subset, which has its own ceiling below.
const BUDGET = 150 * 1024;
// Measured as Brotli, which is how Vercel serves it.
const CABINET_BUDGET = 160 * 1024;
const OUTSIDE = [/\/assets\/doom-attract\.json$/, /\/doom-wasm\//, /\/vendor\/three\.min\.js$/, /\/cabinet\.js$/, /\/models\//];

test("the page weighs under its budget", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm_bg.wasm"), "needs scripts/build-wasm.sh");
  const sizes = new Map<string, number>();
  page.on("response", async (r) => {
    const url = r.url();
    if (!url.startsWith("http://127.0.0.1") || OUTSIDE.some((re) => re.test(url))) return;
    sizes.set(url, (await r.body().catch(() => Buffer.alloc(0))).length);
  });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-scene", /.+/, { timeout: 30_000 });
  await page.waitForLoadState("networkidle");
  const total = [...sizes.values()].reduce((a, b) => a + b, 0);
  expect(total, JSON.stringify(Object.fromEntries(sizes), null, 1)).toBeLessThan(BUDGET);
});

test("the cabinet model stays under 1.5 MB", () => {
  expect(readFileSync(resolve(SITE_DIR, "models/cabinet.glb")).length).toBeLessThan(1.5 * 1024 * 1024);
});

test("the cabinet and its three.js subset stay under their own ceiling, as Brotli", () => {
  const br = ["cabinet.js", "vendor/three.min.js"]
    .map((f) => brotliCompressSync(readFileSync(resolve(SITE_DIR, f))).length)
    .reduce((a, b) => a + b, 0);
  expect(br).toBeLessThan(CABINET_BUDGET);
});
