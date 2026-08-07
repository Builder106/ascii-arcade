import { test, expect } from "@playwright/test";
import { statSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const SITE = join(__dirname, "../../../site");
const LIMIT = 150 * 1024;

test("the page stays inside its 150 kB budget", () => {
  const counted = [
    "index.html",
    "styles.css",
    "main.js",
    "renderer.js",
    "dissolve.js",
    "engine.js",
    "doom.js",
    "motion.js",
    "enhance.js",
  ];

  let total = counted.reduce((n, f) => {
    const p = join(SITE, f);
    return existsSync(p) ? n + statSync(p).size : n;
  }, 0);

  const vendorMotion = join(SITE, "vendor/motion.min.js");
  if (existsSync(vendorMotion)) {
    total += statSync(vendorMotion).size;
  }

  const fontsDir = join(SITE, "fonts");
  if (existsSync(fontsDir)) {
    for (const f of readdirSync(fontsDir)) {
      if (f.endsWith(".woff2")) total += statSync(join(fontsDir, f)).size;
    }
  }

  const pkgDir = join(SITE, "pkg");
  if (existsSync(pkgDir)) {
    for (const f of readdirSync(pkgDir)) {
      if (f.endsWith(".wasm") || f.endsWith(".js")) {
        total += statSync(join(pkgDir, f)).size;
      }
    }
  }

  // DOOM payloads are deliberately outside this: they load only on request.
  expect(total).toBeLessThan(LIMIT);
});
