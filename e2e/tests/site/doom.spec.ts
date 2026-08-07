import { test, expect } from "@playwright/test";

test("the cold open plays real DOOM frames", async ({ page }) => {
  await page.goto("/site/");
  await expect
    .poll(async () => (await page.locator("#doomFrame").innerHTML()).length, {
      timeout: 10000,
    })
    .toBeGreaterThan(500);
});

test("the loop advances rather than holding one still", async ({ page }) => {
  await page.goto("/site/");
  await expect
    .poll(async () => (await page.locator("#doomFrame").innerHTML()).length, {
      timeout: 10000,
    })
    .toBeGreaterThan(500);

  const first = await page.locator("#doomFrame").innerHTML();
  await expect
    .poll(async () => await page.locator("#doomFrame").innerHTML(), { timeout: 4000 })
    .not.toBe(first);
});

test("a missing recording fails loudly on click, not silently", async ({ page }) => {
  await page.route("**/doom-attract.json", (r) => r.abort());
  await page.goto("/site/");
  await page.getByRole("button", { name: /play it/i }).click();
  await expect(page.getByRole("status")).toContainText(/run \.\/scripts\/setup\.sh|clone the repo/i);
});
