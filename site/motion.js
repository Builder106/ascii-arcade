/*
 * Scroll choreography, no library.
 *
 * Reveals are CSS where the browser has scroll-driven animations, and a small
 * IntersectionObserver where it does not. Nothing is hidden until script has
 * run and taken responsibility for showing it again, so a page whose script
 * fails still reads.
 *
 * The hero's recede was originally JS too — a lerped --open custom property,
 * chasing scrollY on the frame loop. That's a phase-lagged follower by
 * construction: against a real trackpad gesture, whose OS-level momentum can
 * keep moving the scroll position for the better part of a second, the lerp
 * spends the whole gesture chasing a moving target, not just settling after
 * it stops. styles.css's #stack reveal, driven by animation-timeline: view(),
 * has no such lag — it's evaluated on the compositor in lockstep with actual
 * scroll position — so the two were visibly out of sync: the next section
 * would finish arriving while the hero was still fading. --open now gets the
 * same native treatment where the browser supports it (styles.css's
 * `@supports (animation-timeline: scroll())` block); the lerp here survives
 * only as the fallback for browsers that don't.
 */

const PAGE_LERP = 0.09;
const OPEN_LERP = 0.22;

const SUPPORTS_SCROLL_TIMELINE =
  typeof CSS !== "undefined" && CSS.supports("animation-timeline", "scroll()");

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
 * Called once per animation frame from main.js. Always publishes `--scroll`
 * for the whole document. Publishes `--open` too, but only when the browser
 * lacks native scroll timelines — where they exist, CSS drives the cold
 * open's recede directly and this value is never read.
 */
export function updateScrollProgress() {
  const max = document.documentElement.scrollHeight - innerHeight;
  const page = max > 0 ? Math.min(1, scrollY / max) : 0;
  smoothedPage += (page - smoothedPage) * PAGE_LERP;

  const root = document.documentElement.style;
  root.setProperty("--scroll", smoothedPage.toFixed(4));

  if (!SUPPORTS_SCROLL_TIMELINE) {
    const open = innerHeight > 0 ? Math.min(1, scrollY / innerHeight) : 0;
    smoothedOpen += (open - smoothedOpen) * OPEN_LERP;
    root.setProperty("--open", smoothedOpen.toFixed(4));
  }

  return smoothedPage;
}

export function scrollProgress() {
  return smoothedPage;
}
