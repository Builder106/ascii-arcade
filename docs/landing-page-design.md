# Landing page design

Design for the ASCII Arcade marketing site, which lives in `site/`.

An earlier version was built and thrown away. Nothing was broken about it; it was
just ordinary. Hero, then a grid of scene cards, then a row of theme swatches,
then install panels. Every section was a list of things, and the ASCII sat on top
as a skin. This document exists so the rebuild doesn't land in the same place.

## What the site is for

Two jobs at once. It should be worth looking at on its own terms, the sort of
page you'd link from a resume or submit to Awwwards, while still getting a macOS
user to the DMG. Narrative sites usually pay for their ambition by burying the
call to action, so this one puts a real action at the end of each movement
instead.

## The conceit

Three things about this product are worth a story, and the old page spent all
three as bullet points.

It draws behind your windows. Not a screensaver that takes over, not a still
image, but a live process sitting in the desktop layer.

macOS can't screenshot it. `⌘⇧3` samples the wallpaper compositor and skips the
window entirely, which is why the app carries its own capture path built on
`CGWindowListCreateImage`.

And at the bottom of that stack, DOOM is playable while you work.

So the page is a descent through the macOS window stack, and DOOM opens it.

### Structure

It opens cold: full-bleed ASCII DOOM, already running, no chrome and no
navigation. One line of copy states the absurd thing plainly, that this is a
desktop background. Two affordances, `PLAY IT` and a quiet download.

The first scroll gesture doesn't scroll. It pulls back. Windows slam in over the
DOOM frame and the view rotates to see the stack edge-on, with DOOM now a bright
line at the bottom. The title lands here, and so does the screenshot beat: the
page takes a screenshot and it comes back with that layer missing. That's real
behaviour rather than a visual metaphor, and it's the best mystery hook the
product has.

Then six movements, indexed by the luminance ramp instead of by number.

| Mark | Heading | Content |
| --- | --- | --- |
| `.` | macOS can't screenshot this | The window layer, and why the OS looks through it |
| `~` | It lives under everything | Where ASCII Arcade sits. Live donut takes the viewport |
| `:` | DOOM was the sideshow | Six states of one renderer; DOOM demotes itself |
| `=` | Every glyph keys off one colour | Four palettes; picking one rewrites the page |
| `*` | It doesn't need a desktop | Terminal and browser, same core |
| `@` | Put it on yours | Install, Gatekeeper, call to action |

Leading with DOOM would normally cost you the reveal. Here it just moves it. You
meet DOOM as spectacle, then learn at `:` that it's one scene out of six, off by
default and deliberately kept out of the cycle. The turn isn't that there's a
shooter down there. It's that the shooter was the least interesting thing running.

### The index is the luminance ramp

Section markers are `.,-~:;=!*#$@`, the ramp out of `DonutFrameGenerator.swift`,
where glyph density already encodes depth. You start at `.` with nothing resolved
and arrive at `@` with the layer fully drawn. The fixed rail carries the whole
ramp with the current position lit, so the progress indicator is built out of the
product's own primitive.

Numbered section labels (`01 —`, `02 —`) are ruled out specifically. So is
stacking an eyebrow label above each heading, and so is any run of headings
sharing one grammatical shape. The six above vary on purpose.

## Rendering

### One renderer, not six thumbnails

A single character grid fixed to the viewport, sitting behind all content, whose
state is driven by scroll position. Sections float over it. The previous site's
six small canvases were feature-grid thinking wearing ASCII. Here the page
background is the product running, and scrolling is the scene switcher.

### Canvas, because of per-cell colour

A `<pre>` rewritten each frame handles glyphs fine. But Matrix needs bright heads
with fading trails, Pipes carries per-pipe hue, and DOOM is full colour, and
per-cell colour in the DOM works out to roughly 9,600 spans a frame. So the live
grid is canvas, `aria-hidden`, decorative, with a real text alternative.

Anything typographic stays real DOM text: the depth rail, section rules, ramp
gradients, the cold-open wordmark. If it could be text, it isn't painted as
pixels.

### The engine is `aa-core`, compiled to WebAssembly

`crates/aa-core` has zero dependencies, calls itself platform-neutral, and ships
its own `rng.rs` so it doesn't even pull in `rand`. All five maths scenes already
live in `crates/aa-core/src/scenes/`. It compiles to `wasm32-unknown-unknown`
without a fight.

This matters for a reason beyond novelty. The old site hand-ported the Swift into
JavaScript and its README carried the warning: *"If you change those in Swift,
change them here too or the page stops telling the truth about the product."*
That's a drift bug written permanently into the design. Running `aa-core` as
WebAssembly removes the possibility, because the site is executing the shipping
engine rather than a second copy of it that has to be kept in sync by hand.

Rust stops at the engine. The DOM, scroll handling, and canvas calls stay in
JavaScript. A six-section page has almost no state worth managing, so a Rust UI
framework would inflate the bundle by an order of magnitude and then get in the
way of scroll choreography, which is DOM and CSS work anyway.

### The scene interface mirrors the app's own

`Sources/AsciiArcadeCore/AsciiScene.swift` already defines the right abstraction,
so the site borrows it instead of inventing one:

```text
setGrid(cols, rows)        resizable character grid
frame(t) -> glyphs         pull-based, per-scene maths
coloredFrame(t) -> cells   optional per-cell RGB; null means paint in theme colour
applyBaseColor(rgb)        Matrix turns amber under the Amber theme
fixedGrid                  non-null means scaled colour bitmap, not glyphs on the grid
```

`fixedGrid` is easy to skim past and load-bearing. A scene with a fixed pixel
resolution, which means DOOM, gets painted as a scaled colour bitmap rather than
as font glyphs bound to the text grid. DOOM composites differently from the maths
scenes and the site should keep that instead of flattening it.

### Transitions are character dissolves

Switching scenes doesn't crossfade opacity. Each cell holds a stable random
threshold in a `Float32Array`, and as transition progress sweeps 0 to 1, cells
flip from one scene's glyph to the other's as progress passes their threshold.
The grid dissolves character by character. One array, one comparison per cell,
and it's something you can't do with images.

### DOOM is a frame source

DOOM implements the same interface as everything else, which is how the app
already treats it. Two implementations sit behind one contract: `RecordedDoom`
now, `WasmDoom` later, with no other code changes.

The cold open plays a short loop of DOOM's own attract-mode demo, the sequence it
runs by itself when idle. It's authentic to the software, it's small, and it
autoplays as the hook. `PLAY IT` is what loads the heavy interactive path, so
nothing downloads until somebody asks for it.

The page says outright that this one scene is a recording while the rest of the
grid is live. Given the honesty rule the old site wrote for itself, the hook had
better not be the one thing on the page that's faking it.

## Motion

Motion (motion.dev), vendored as a committed file. It's MIT licensed, so it
doesn't conflict with this repository's GPL-2.0, which decides it. GSAP has the
better pinning ergonomics, but it ships under GreenSock's own licence, and this
project has been careful about exactly that kind of question. Motion is also
about half the size and uses native browser APIs where they exist.

Motion earns its bytes on the pinned pull-back and on reveal ergonomics. Scrub
smoothing comes from the engine's own `requestAnimationFrame` loop, which is
already running for the canvas. A lerped scroll-progress value driving transforms
out of that loop gives the inertia native scroll timelines can't, since those are
strictly one-to-one with scroll position.

CSS keeps everything it can hold. Scroll position drives the renderer through an
`IntersectionObserver` that reads each section's `data-scene` and sets a target,
and the `rAF` loop interpolates toward it. Decoupling transitions from scroll
velocity means scrubbing back and forth doesn't thrash, and there are no scroll
event listeners anywhere.

Browser support is progressive enhancement rather than a floor. Visitors without
native scroll timelines get the static layout, which is the same complete page
that `prefers-reduced-motion` produces.

## Accessibility

Under `prefers-reduced-motion: reduce` the pull-back doesn't run. The page opens
already stacked, so the composition survives without the camera move. Canvases
paint one frame with no loop, dissolves become instant swaps, and the DOOM cold
open is a still. The copy was always doing the work of the assertion, so nothing
is lost, and nothing becomes unreachable.

The canvas is `aria-hidden` and decorative, and every fact it carries also exists
in text. One detail needs deliberate care: `.` `~` `:` `=` `*` `@` mean nothing to
a screen reader, so the depth rail is a real `<nav>` of anchors and each glyph
takes its section's heading as its accessible name. Scene and theme controls are
real buttons in a labelled group, keyboard-operable, with visible focus.

Body text holds 4.5:1 contrast across all four palettes, Ghost included, and this
one is worth measuring rather than eyeballing. The old site shipped muted tones at
4.09:1 and 2.32:1 before measurement caught them. Pure `#000000` also reads
synthetic, so the Hacker background is tinted fractionally toward the phosphor hue.

The DOOM panel has one hazard that isn't obvious until it bites: arrow keys scroll
the page. It captures keys only while focused, says so, exits on Escape, never
traps focus, and stays opt-in, which matches the app's own position on not putting
a shooter in front of people who didn't ask for one.

## Conversion

Where the actions sit, and why:

The cold open carries `PLAY IT` plus a quiet download. After the screenshot beat
there's nothing at all, because the mystery needs a moment and a call to action
there would step on it. Download comes back at `~`, once the value has actually
landed, and again at `:`. Section `*` converts differently, with
copy-to-clipboard command blocks, because that reader wants a terminal rather
than a disk image. Section `@` is the full install.

The persistent affordance is the current depth glyph and a download link. No
pulsing dot, no live badge, no status indicator.

Gatekeeper gets said out loud rather than buried. The app is unsigned and macOS
blocks the first launch. Explaining that it's down to there being no paid Apple
Developer account, and giving both the right-click-to-open path and the `xattr`
command, turns the likeliest bounce into evidence that the page is being straight
with the reader.

## Build, budget, deployment

The site is no longer zero-build. Compiling `aa-core` adds a Rust and
`wasm-bindgen` step, which runs on `ampere-dev` and never on the Mac, per the
repository's working rules. Motion is vendored as a committed file rather than
installed, so there's still no package manager and no `node_modules`.

The site ships one self-hosted typeface, IBM Plex Mono, subset to the glyphs it
actually uses. That comes to printable ASCII plus exactly seven characters: `█`
from Life and `─ │ ┌ ┐ └ ┘` from Pipes. Around 102 glyphs, which lands well
under 10 kB as woff2. Subsetting follows the `font-workflow` process.

The platform monospace stack was the obvious cheaper answer and it's the wrong
one here, for a reason specific to Pipes. Box-drawing characters only look like
pipes if `─` spans the full advance width and `│` spans the full line height, so
that adjacent cells join. Whether they do is a decision the type designer made,
and system monospace fonts disagree about it. Menlo joins cleanly, plenty of
Linux fallbacks leave visible gaps, and a gap turns one of the six scenes into
visible breakage on a page whose whole argument is that this is really running.
Ten kilobytes buys the same grid on every machine, and it lets canvas cell
metrics be tuned once against a known face instead of measured against whatever
turned up.

The same face sets the body copy, which keeps the page consistent with its own
claim about being made of characters.

HTML, CSS, JavaScript, and the WebAssembly module together stay under roughly
150 kB. DOOM payloads sit outside that budget and load only on request.

There's no GitHub Pages workflow in the repository today, so deployment needs
adding as part of this work.

## When things fail

Every moving part on this page is an enhancement over something that already
reads, and that's deliberate rather than a happy accident.

If the WebAssembly module fails to load or the browser blocks it, the canvas
stays empty. It's `aria-hidden` decoration and every fact it illustrates is also
written in text, so the page loses its wallpaper and keeps its argument. The
static layout underneath is the same one `prefers-reduced-motion` gets, so it's a
path that gets exercised regularly rather than a fallback nobody has looked at
since it was written.

If the DOOM recording fails to fetch, the cold open holds on a single still
frame and the copy carries the claim on its own. If Motion fails to load, the
CSS reveals still run and the pinned pull-back degrades to an ordinary scroll.
If the browser has no native scroll timelines, same outcome.

The one failure worth handling loudly is `PLAY IT`. Someone clicked it on
purpose, so a silent no-op is the wrong answer. It reports that the interactive
build didn't load and points at running it locally.

## Testing and CI tier

`crates/aa-core` already carries Rust unit tests across `donut.rs`, `helix.rs`,
`life.rs`, `matrix.rs`, `mod.rs`, `frame.rs`, `color.rs`, `ansi.rs`, and
`rng.rs`. Compiling that crate to WebAssembly means the site runs code the test
suite already covers, so the scene maths needs no new tests. The binding layer is
thin enough to check at the seams instead.

Against the standard pipeline this feature moves three tiers and deliberately
leaves the rest alone:

Build verification gains the `wasm32-unknown-unknown` target, so a change that
breaks the WebAssembly build fails CI rather than the deploy.

E2E gains real coverage. `e2e/` already runs Playwright and `playwright-bdd`, but
it's pointed at demo recordings today, not at the site. This adds specs for the
scroll narrative reaching every section, the reduced-motion path rendering a
complete page, and the WASM-absent path doing the same.

Accessibility becomes an automated gate, which means adding
`@axe-core/playwright`, since it isn't in `e2e/package.json` today. Contrast is
the reason: this document commits to 4.5:1 across four palettes, and the previous
site shipped 4.09:1 and 2.32:1 precisely because a person eyeballed it. A number
that specific should be enforced by something that can count.

Nothing else moves. There's no new API route or external consumer, so integration
and contract testing stay where they are. There's no new invariant worth stating
as a property, since the scene generators already have their own tests upstream.
Web Vitals stays ungated, with one exception: the 150 kB budget in this document
is a claim, so a bundle-size check enforces it. If it's ever breached, something
regressed, and the fix isn't to raise the ceiling.

## Out of scope

The WASM port of `doom_ascii` is a separate project. It needs an emscripten
toolchain, a doomgeneric build, a JavaScript-to-framebuffer bridge, and a WAD
strategy, since Freedoom's IWADs are 27 MB each. It also reverses a licensing
position this repository holds on purpose: `doom_ascii` is GPL-2.0 and is
currently built locally rather than redistributed, gated behind `INCLUDE_DOOM=1`.
Shipping a WASM build makes the site a distributor, with the source-offer
obligations that brings.

That reversal deserves its own design document and its own decision, not a
side effect of a landing page. `RecordedDoom` ships first, `WasmDoom` swaps in
behind the same interface when it's ready, and the page never blocks on it.

## A bug found while specifying this

Not a site issue, recorded here so it doesn't get lost.

`crates/aa-render/src/font.rs` embeds an 8x16 bitmap font covering printable
ASCII `0x20..=0x7E`, and `glyph_bitmap()` in `lib.rs` returns `None` for anything
outside that range, which the caller skips. Life's `█` and all six of Pipes'
box-drawing characters are outside it. On the native Rust wallpaper shells
(`shells/linux`, `shells/windows`) both scenes should therefore render blank or
close to it.

The other paths hide this. The macOS shell uses
`NSFont.monospacedSystemFont`, which has the glyphs. `aa play` and `aa-web` emit
ANSI and let the terminal or xterm.js supply the font. Only the shells that
rasterise through `aa-render` hit the missing-glyph path.

This was read off the source, not reproduced on a Linux shell, so it wants
confirming before anyone acts on it. The fix is seven more bitmaps in
`gen_font.py` and a wider range check.
