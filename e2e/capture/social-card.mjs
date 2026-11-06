// Renders assets/social-card.html to site/assets/social-card.png, the 1200x630
// image behind the site's og:image tag. Run from e2e/: npm run social-card
import { chromium } from "@playwright/test";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const source = pathToFileURL(resolve(root, "assets/social-card.html")).href;
const out = resolve(root, "site/assets/social-card.png");

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 });
await page.goto(source);
await page.evaluate(() => document.fonts.ready);
await page.screenshot({ path: out, clip: { x: 0, y: 0, width: 1200, height: 630 } });
await browser.close();
console.log(`wrote ${out}`);
