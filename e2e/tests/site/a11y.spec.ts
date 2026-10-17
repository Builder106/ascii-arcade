import AxeBuilder from "@axe-core/playwright";
import { test, expect, open, STATIONS, THEMES } from "./fixtures";

// The wall is aria-hidden decoration; every fact it shows is also in text.
async function scan(page: import("@playwright/test").Page) {
  const { violations } = await new AxeBuilder({ page }).exclude("#wall").analyze();
  return violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

test.use({ contextOptions: { reducedMotion: "reduce" } });

for (const theme of THEMES) {
  test(`every station passes axe in ${theme}`, async ({ page }) => {
    await page.addInitScript((t) => localStorage.setItem("aa-theme", t), theme);
    for (const id of STATIONS) {
      await open(page, `/#${id}`);
      await expect(page.locator(`#${id}`)).toBeVisible();
      expect(await scan(page), `${theme} #${id}`).toEqual([]);
    }
  });
}

test("the 404 passes axe", async ({ page }) => {
  await page.goto("/404.html");
  expect(await scan(page)).toEqual([]);
});

test("the skip link comes first and lands on the content", async ({ page }) => {
  await open(page);
  await page.keyboard.press("Tab");
  await expect(page.locator(".skip")).toBeFocused();
  await expect(page.locator(".skip")).toBeInViewport();
});

test("every station's copy is in the HTML without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto("/");
  for (const id of STATIONS) await expect(page.locator(`#${id}`)).toBeVisible();
  await expect(page.locator("h1")).toHaveCount(1);
  await context.close();
});
