/*
 * Scroll choreography, no library.
 *
 * Reveals are CSS where the browser has scroll-driven animations, and a small
 * IntersectionObserver where it does not. Nothing is hidden until script has
 * run and taken responsibility for showing it again, so a page whose script
 * fails still reads.
 *
 * The scrub is the one thing CSS cannot do. Native scroll timelines track the
 * scrollbar exactly; the inertia that makes scroll-linked motion feel
 * deliberate comes from lerping toward the real position on the frame loop the
 * canvas is already running.
 */

const LERP = 0.09;

let smoothedPage = 0;
let smoothedOpen = 0;

export function initMotion({ reduced }) {
  if (reduced) return;

  // CSS owns the reveals when scroll timelines exist.
  if (CSS.supports("animation-timeline", "view()")) return;

  const root = document.documentElement;
  root.classList.add("js-reveal");

  const seen = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          e.target.classList.add("is-in");
          seen.unobserve(e.target);
        }
      }
    },
    { threshold: 0.15, rootMargin: "0px 0px -8% 0px" },
  );

  for (const el of document.querySelectorAll(".sec")) {
    seen.observe(el);
  }
}

/**
 * Called once per animation frame from main.js. Publishes two values:
 * `--scroll` for the whole document and `--open` for the first viewport, which
 * is what drives the cold open receding as the stack closes over it.
 */
export function updateScrollProgress() {
  const max = document.documentElement.scrollHeight - innerHeight;
  const page = max > 0 ? Math.min(1, scrollY / max) : 0;
  const open = innerHeight > 0 ? Math.min(1, scrollY / innerHeight) : 0;

  smoothedPage += (page - smoothedPage) * LERP;
  smoothedOpen += (open - smoothedOpen) * LERP;

  const root = document.documentElement.style;
  root.setProperty("--scroll", smoothedPage.toFixed(4));
  root.setProperty("--open", smoothedOpen.toFixed(4));

  return smoothedPage;
}

export function scrollProgress() {
  return smoothedPage;
}
