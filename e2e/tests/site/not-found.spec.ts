import { test, expect } from "./fixtures";

test("the 404 page leads back to the arcade", async ({ page }) => {
  await page.goto("/404.html");
  await expect(page.locator("h1")).toHaveText("Nothing is plugged in here.");
  await expect(page.getByRole("link", { name: "Back to the arcade" })).toHaveAttribute("href", "/");
});
