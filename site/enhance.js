/*
 * Additions to markup that already works. The command blocks are readable and
 * selectable without a copy button; the dock is a shortcut, not the only route
 * to the download. Nothing here is required for the page to make sense.
 */

const DEPTHS = [
  ["stack", "."],
  ["layer", "~"],
  ["scenes", ":"],
  ["palette-section", "="],
  ["surfaces", "*"],
  ["install", "@"],
];

export function initEnhancements() {
  addCopyButtons();
  trackDepth();
}

function addCopyButtons() {
  if (!navigator.clipboard) return;

  for (const block of document.querySelectorAll("pre.code")) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy";
    btn.textContent = "Copy";

    btn.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(block.innerText.trim());
        btn.textContent = "Copied";
      } catch {
        // Denied permission or an insecure context. Say so rather than
        // leaving a button that silently does nothing.
        btn.textContent = "Select and copy";
      }
      setTimeout(() => {
        btn.textContent = "Copy";
      }, 1800);
    });

    block.after(btn);
  }
}

function trackDepth() {
  const out = document.querySelector("[data-depth]");
  if (!out) return;

  const marks = new Map(DEPTHS);
  const watcher = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting && e.intersectionRatio > 0.5) {
          out.textContent = marks.get(e.target.id) ?? ".";
        }
      }
    },
    { threshold: [0.5] },
  );

  for (const [id] of DEPTHS) {
    const el = document.getElementById(id);
    if (el) watcher.observe(el);
  }
}
