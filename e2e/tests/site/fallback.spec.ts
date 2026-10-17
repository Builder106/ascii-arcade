import { test, expect, open, mode } from "./fixtures";

test.use({ allowConsole: [/Failed to load resource/] });

test("without the aa-core build the page still reads and runs its loop", async ({ page }) => {
  await page.route("**/pkg/**", (r) => r.abort());
  await page.route("**/cabinet.js", (r) => r.abort());
  await page.clock.install();
  await open(page);
  await expect(page.locator("h1")).toHaveText("ASCII Arcade");
  await page.clock.runFor(9_500);
  await expect(page.locator("#demo")).toBeVisible();
  await page.keyboard.press("Shift");
  expect(await mode(page)).toBe("player");
});

test("without the DOOM recording the demo station keeps its copy", async ({ page }) => {
  await page.route("**/assets/doom-attract.json", (r) => r.abort());
  await open(page, "/#demo");
  await expect(page.locator("#demo h2")).toBeVisible();
  await expect(page.locator("#doom-note")).toBeVisible();
});
