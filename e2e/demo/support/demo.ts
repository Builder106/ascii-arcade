import type { Page } from "@playwright/test";

/** Explicit pause after navigations/assertions so the viewer can register a beat. */
export async function dwellForDemo(page: Page, ms = Number(process.env.DEMO_DWELL_MS ?? 1500)) {
  if (process.env.DEMO !== "1") return;
  try {
    await page.waitForTimeout(ms);
  } catch {
    /* page already closed */
  }
}

/** Inject a visible cursor dot so the viewer can see where the test is "looking". */
export async function injectCursor(page: Page) {
  await page.addInitScript(() => {
    const dot = document.createElement("div");
    dot.style.cssText =
      "position:fixed;z-index:2147483647;width:16px;height:16px;margin:-8px 0 0 -8px;" +
      "border-radius:50%;background:rgba(51,255,102,.9);box-shadow:0 0 12px rgba(51,255,102,.8);" +
      "pointer-events:none;transition:transform .05s linear;";
    const add = () => document.body && document.body.appendChild(dot);
    if (document.body) add();
    else window.addEventListener("DOMContentLoaded", add);
    window.addEventListener("mousemove", (e) => {
      dot.style.left = e.clientX + "px";
      dot.style.top = e.clientY + "px";
    });
  });
}
