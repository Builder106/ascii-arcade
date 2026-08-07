import { test, expect } from "@playwright/test";

const SECTIONS = ["stack", "layer", "scenes", "palette-section", "surfaces", "install"];

test("scrolling reaches every section", async ({ page }) => {
  await page.goto("/site/");
  for (const id of SECTIONS) {
    await page.locator(`#${id}`).scrollIntoViewIfNeeded();
    await expect(page.locator(`#${id}`)).toBeInViewport();
  }
});

test("content is visible without JavaScript", async ({ browser }) => {
  const ctx = await browser.newContext({ javaScriptEnabled: false });
  const page = await ctx.newPage();
  await page.goto("/site/");

  // Nothing may be hidden by default: the reveal class is added by script, so
  // a page whose script never runs still reads.
  for (const id of SECTIONS) {
    const opacity = await page
      .locator(`#${id}`)
      .evaluate((el) => getComputedStyle(el).opacity);
    expect(Number(opacity)).toBe(1);
  }
  await ctx.close();
});

test("reduced motion still renders the whole page", async ({ browser }) => {
  const ctx = await browser.newContext({ reducedMotion: "reduce" });
  const page = await ctx.newPage();
  await page.goto("/site/");

  await expect(page.getByRole("heading", { name: "Put it on yours" })).toBeAttached();
  await page.locator("#install").scrollIntoViewIfNeeded();
  const opacity = await page
    .locator("#install")
    .evaluate((el) => getComputedStyle(el).opacity);
  expect(Number(opacity)).toBe(1);

  await ctx.close();
});

test("the scroll scrub publishes a smoothed progress value", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, { timeout: 15000 });

  const read = () =>
    page.evaluate(() =>
      Number(
        getComputedStyle(document.documentElement).getPropertyValue("--scroll"),
      ),
    );

  expect(await read()).toBeLessThan(0.05);

  await page.locator("#install").scrollIntoViewIfNeeded();
  await page.waitForTimeout(900);

  const after = await read();
  expect(after).toBeGreaterThan(0.3);
  expect(after).toBeLessThanOrEqual(1);
});
