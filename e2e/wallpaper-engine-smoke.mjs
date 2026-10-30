import { chromium } from "playwright";
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 640, height: 480 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  const url = process.argv[2] ? pathToFileURL(resolve(process.argv[2], "index.html")).href : "http://127.0.0.1:8898/";
  await page.goto(url);
  const pixels = () => page.locator("canvas").evaluate((canvas) => {
    const { data } = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height);
    return Array.from(data).filter((value, index) => index % 4 !== 3 && value > 0).length;
  });
  await page.waitForFunction(() => {
    const canvas = document.querySelector("canvas");
    return canvas && canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data.some((value, index) => index % 4 !== 3 && value > 0);
  });
  assert.ok(await pixels() > 50);
  await page.evaluate(() => window.wallpaperPropertyListener.setPaused(true));
  const before = await page.locator("canvas").evaluate((canvas) => canvas.toDataURL());
  await page.waitForTimeout(250);
  assert.equal(await page.locator("canvas").evaluate((canvas) => canvas.toDataURL()), before);
  await page.evaluate(() => window.wallpaperPropertyListener.setPaused(false));
  await page.waitForFunction((previous) => document.querySelector("canvas").toDataURL() !== previous, before);
  await page.setViewportSize({ width: 800, height: 400 });
  await page.waitForFunction(() => document.querySelector("canvas").width === 800 * devicePixelRatio);
  assert.deepEqual(errors, []);
  console.log("Wallpaper renders, pauses, resumes, and resizes in Chromium");
} finally {
  await browser.close();
}
