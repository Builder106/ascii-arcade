# doom-wasm patches

Everything in this directory targets `wojciech-graj/doom-ascii` pinned to
commit `b5188d7c9c4da6c81264a7803e8725ac3df2cfea` (`Release 0.3.1`,
2025-07-21). This pin is what `scripts/build-doom-wasm.sh` checks out and
what the GPL-2.0 source-offer link in the site's footer/dock points at —
it must never silently float to a newer commit. Bumping it is a deliberate,
reviewed action: re-verify every file this directory touches still applies
cleanly and every line number `doomgeneric_wasm.c`'s comments reference is
still accurate.

- `doomgeneric_wasm.c` — new platform backend implementing doomgeneric's

  `DG_*` interface for a browser instead of a terminal. Not a patch; a
  whole new file, copied into the pinned clone's `src/` at build time.

- `main-loop.patch`— the`emscripten_set_main_loop` restructuring of

  `src/d_main.c`'s `D_DoomLoop`, guarded behind `#ifdef __EMSCRIPTEN__` so
  the native build (`scripts/setup.sh`) is unaffected.

See `docs/doom-wasm-design.md` for the full design and
`docs/doom-wasm-plan-a.md` for how these get built and verified.
