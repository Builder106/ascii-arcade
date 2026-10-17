import { test, expect, open, built, waitForScene, litPixels, mode } from "./fixtures";

// These specs cover DOOM on the flat page. The cabinet is blocked: on the
// test machine's software GPU it loads slowly enough for the page to go idle
// and move off the DOOM station. cabinet.spec.ts covers DOOM on the cabinet.
test.use({ allowConsole: [/Failed to load resource/] });
test.beforeEach(async ({ page }) => {
  await page.route("**/cabinet.js", (r) => r.abort());
});

test("the demo station plays the recorded attract loop", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  await open(page, "/#demo");
  await waitForScene(page, "doom");
  await expect.poll(() => litPixels(page)).toBeGreaterThan(1000);
});

test("Play DOOM loads the real build and hands the keyboard back on Esc", async ({ page }) => {
  test.skip(!built("doom-wasm/doom.js"), "needs scripts/build-doom-wasm.sh");
  test.setTimeout(120_000);
  await open(page, "/#demo");
  await page.locator("#play-doom").click();
  await waitForScene(page, "doom-live");
  expect(await mode(page)).toBe("doom");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  expect(await mode(page)).toBe("player");
  await expect(page.locator("#doom-status")).toContainText("keeps running");
});

test.describe("when the playable build is missing", () => {
  test.use({ allowConsole: [/Failed to load resource/] });

  test("Play DOOM says so instead of doing nothing", async ({ page }) => {
    await page.route("**/doom-wasm/**", (r) => r.abort());
    await open(page, "/#demo");
    await page.locator("#play-doom").click();
    await expect(page.locator("#doom-status")).toContainText("didn’t load");
    expect(await mode(page)).toBe("player");
  });
});

test.describe("on a phone", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("Play DOOM is replaced by a note that it needs a keyboard", async ({ page }) => {
    await open(page, "/#demo");
    await expect(page.locator("#play-doom")).toBeHidden();
    await expect(page.locator("#demo .touch-only")).toBeVisible();
  });
});
