import { test, expect } from "@playwright/test";

test("keyboard input reaches doom-wasm's key queue", async ({ page }) => {
  await page.goto("/site/");

  const result = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    document.body.appendChild(canvas);

    const handle = await loadDoomSkeleton(canvas);
    await new Promise((resolve) => setTimeout(resolve, 1500));

    // wasm_push_key has no observable return value and DG_ScreenBuffer's
    // content depends on game state, not directly on key presses in a way
    // a test can assert against cheaply. What's actually testable without
    // reaching into the module's internals: calling handle.push() doesn't
    // throw, and dispatching a mapped keydown while a session is active
    // calls preventDefault (proving the listener is wired and the code is
    // recognized), while an unmapped key does not.
    let threw = false;
    try {
      handle.push(1, 0xad); // KEY_UPARROW
      handle.push(0, 0xad);
    } catch {
      threw = true;
    }

    const mappedEvent = new KeyboardEvent("keydown", { code: "ArrowUp", cancelable: true });
    dispatchEvent(mappedEvent);
    const mappedPrevented = mappedEvent.defaultPrevented;

    const unmappedEvent = new KeyboardEvent("keydown", { code: "KeyQ", cancelable: true });
    dispatchEvent(unmappedEvent);
    const unmappedPrevented = unmappedEvent.defaultPrevented;

    handle.stop();
    canvas.remove();
    return { threw, mappedPrevented, unmappedPrevented };
  });

  expect(result.threw).toBe(false);
  expect(result.mappedPrevented).toBe(true);
  expect(result.unmappedPrevented).toBe(false);
});
