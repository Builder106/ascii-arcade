import { test, expect, open, built } from "./fixtures";

test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");

const opacity = (page: import("@playwright/test").Page, sel: string) =>
  page.locator(sel).evaluate((el) => Number(getComputedStyle(el).opacity));

test.describe("on the flat page", () => {
  // Without the cabinet the log only waits for the engine, so this measures
  // the boot itself against its 2.4 s cap.
  test.use({ allowConsole: [/Failed to load resource/] });

  test("the power-on shows the headline at once, then hands over the page within its cap", async ({ page }) => {
    await page.route("**/cabinet.js", (r) => r.abort());
    await open(page, "/", { boot: true });
    const html = page.locator("html");
    await expect(html).toHaveClass(/booting/);
    await expect(page.locator("h1")).toBeVisible();
    expect(await opacity(page, ".strip")).toBe(0);
    await expect(html).not.toHaveClass(/booting/, { timeout: 4_000 });
    await expect.poll(() => opacity(page, ".strip")).toBe(1);
    await expect(page.locator("#power")).toHaveCount(0);
  });

  test("the power-on plays again on reload", async ({ page }) => {
    await page.route("**/cabinet.js", (r) => r.abort());
    await open(page, "/", { boot: true });
    await expect(page.locator("html")).not.toHaveClass(/booting/, { timeout: 4_000 });
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-mode", /.+/);
    await expect(page.locator("html")).toHaveClass(/booting/);
  });
});

// With the cabinet, the test machine's software GPU can hold the main thread
// for several seconds while it compiles shaders, and the boot only ticks on
// frames, so this allows for that.
test("with the cabinet, the power-on ends on the cabinet in its room", async ({ page }) => {
  test.setTimeout(60_000);
  await open(page, "/", { boot: true });
  await expect(page.locator("html")).not.toHaveClass(/booting/, { timeout: 30_000 });
  await expect(page.locator("html")).toHaveAttribute("data-view", "cabinet", { timeout: 30_000 });
});

test("any key skips the power-on", async ({ page }) => {
  await open(page, "/", { boot: true });
  await expect(page.locator("html")).toHaveClass(/booting/);
  await page.keyboard.press("Shift");
  await expect(page.locator("html")).not.toHaveClass(/booting/, { timeout: 1_000 });
});

test("a deep link goes straight to its station", async ({ page }) => {
  await open(page, "/#install", { boot: true });
  await expect(page.locator("html")).not.toHaveClass(/booting/);
  await expect(page.locator("#install")).toBeVisible();
});

test.describe("reduced motion", () => {
  test.use({ contextOptions: { reducedMotion: "reduce" } });

  test("never powers on", async ({ page }) => {
    await open(page, "/", { boot: true });
    await expect(page.locator("html")).not.toHaveClass(/booting/);
    expect(await opacity(page, ".strip")).toBe(1);
  });
});
