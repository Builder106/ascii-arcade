import { test, expect } from "@playwright/test";

test("command blocks gain a copy button", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, { timeout: 15000 });

  const copy = page.locator("#surfaces").getByRole("button", { name: /copy/i }).first();
  await expect(copy).toBeVisible();
  await copy.click();

  const clip = await page.evaluate(() => navigator.clipboard.readText());
  expect(clip).toContain("cargo run -p aa");
});

test("the persistent affordance tracks depth and offers the download", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, { timeout: 15000 });

  const dock = page.getByRole("complementary", { name: /progress/i });
  await expect(dock.getByRole("link", { name: /download/i })).toBeVisible();

  await page.locator("#install").scrollIntoViewIfNeeded();
  await expect(dock.locator("[data-depth]")).toHaveText("@", { timeout: 5000 });

  await page.locator("#layer").scrollIntoViewIfNeeded();
  await expect(dock.locator("[data-depth]")).toHaveText("~", { timeout: 5000 });
});
