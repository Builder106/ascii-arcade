import { test, expect, open, STATIONS } from "./fixtures";

const SIZES = [
  { name: "phone", width: 390, height: 844 },
  { name: "laptop", width: 1280, height: 800 },
];

for (const size of SIZES) {
  test.describe(size.name, () => {
    test.use({ viewport: { width: size.width, height: size.height }, contextOptions: { reducedMotion: "reduce" } });

    test("no station overflows sideways or hides a target", async ({ page }) => {
      for (const id of STATIONS) {
        await open(page, `/#${id}`);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
        expect(overflow, `#${id}`).toBeLessThanOrEqual(0);
        const small = await page.evaluate(() =>
          [...document.querySelectorAll("a, button")]
            .filter((el) => (el as HTMLElement).offsetParent && !el.closest("p, li:not(.strip li)"))
            .map((el) => [el.textContent?.trim(), el.getBoundingClientRect().height] as const)
            .filter(([, h]) => h > 0 && h < 24),
        );
        expect(small, `#${id}`).toEqual([]);
      }
    });

    test("the current station sits fully inside the window", async ({ page }) => {
      for (const id of STATIONS) {
        await open(page, `/#${id}`);
        const box = await page.locator(`#${id}`).boundingBox();
        expect(box, `#${id}`).not.toBeNull();
        expect(box!.y, `#${id}`).toBeGreaterThanOrEqual(0);
      }
    });
  });
}
