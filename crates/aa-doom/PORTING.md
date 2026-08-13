# Building `doom_ascii` per platform

`aa-doom` shells out to the vendored [`doom_ascii`](https://github.com/wojciech-graj/doom-ascii)
binary (a `doomgeneric`-based terminal DOOM, pure C, GPL — built locally, not
redistributed). `scripts/setup.sh`clones it and runs`make`. The launcher
([`launcher.rs`](src/launcher.rs)) looks for `doom_ascii`(or`doom_ascii.exe`
on Windows) under `bin/`, `$DOOM_ASCII_PATH`, and the system bin dirs.

The DOOM frames reach us over a PTY via `portable-pty`, which abstracts the
per-OS pseudo-terminal: **forkpty**on macOS/Linux,**ConPTY** on Windows. That
single abstraction is why DOOM works on all three.

## Linux

Trivial — same as macOS. `cc`+`make`:

```text
git clone --depth 1 https://github.com/wojciech-graj/doom-ascii
cd doom-ascii && make
```text

No extra libraries: `doom_ascii` only does terminal I/O (ANSI escapes to
stdout). `scripts/setup.sh` runs on Linux under bash (shebang
`#!/usr/bin/env bash`) — verified building an aarch64 binary on Ubuntu 24.04,
where the `aa-doom` end-to-end spawn test then drives it over a forkpty PTY.

## Windows

Needs a C toolchain and produces `doom_ascii.exe`. The upstream `Makefile` is
gcc-oriented, so the path of least resistance is **MSYS2 / MinGW-w64**:

```text
pacman -S --needed make mingw-w64-x86_64-gcc
git clone --depth 1 https://github.com/wojciech-graj/doom-ascii
cd doom-ascii && make
```text

Notes:

- **VT output:** `doom_ascii` emits ANSI/truecolor escapes. Under our ConPTY

  master those are delivered as-is to [`screen.rs`](src/screen.rs) — we parse
  the escapes ourselves, so the child doesn't need the Windows console's own VT
  processing enabled.

- **Binary name:** the launcher already prefers `doom_ascii.exe` on Windows.
- An MSVC build is possible but requires porting the Makefile; MinGW is simpler

  and the resulting exe runs fine under ConPTY.

## WADs

Any supported IWAD in `wad/`(or`$DOOM_WAD_DIR`); the repo ships the
redistributable Freedoom set. The launcher exports `DOOMWADDIR`so`doom_ascii`
finds adjacent lumps.
