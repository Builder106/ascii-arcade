import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const THEMES = ["hacker", "amber", "ice", "ghost"];

// Contrast is the reason this is automated. The previous marketing site
// shipped muted tones at 4.09:1 and 2.32:1 because a person eyeballed them.
for (const theme of THEMES) {
  test(`no accessibility violations under the ${theme} palette`, async ({ page }) => {
    await page.goto("/site/");
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
    }, theme);

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .exclude("#doomFrame")
      .analyze();

    expect(results.violations).toEqual([]);
  });
}

test("the canvas is hidden from assistive technology", async ({ page }) => {
  await page.goto("/site/");
  await expect(page.locator("#grid")).toHaveAttribute("aria-hidden", "true");
});

test("the page is reachable by keyboard from the skip link", async ({ page }) => {
  await page.goto("/site/");
  await page.keyboard.press("Tab");
  await expect(page.locator("a.skip")).toBeFocused();
});
