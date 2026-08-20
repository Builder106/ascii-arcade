# ASCII-Arcade Roadmap

Terminal & browser ASCII gaming engine roadmap.

## v1.1 — WebAssembly & Game Porting

- **Doom WASM Integration**: Clean WASM-compiled execution per [`docs/specs/doom-wasm-plan-a.md`](docs/specs/doom-wasm-plan-a.md).
- **Background Shader Modes**: Configurable background opacity and retro CRT shader filters.

## v1.2 — Game Library Expansion

- **Classic Arcade Ports**: Snake, Tetris, Space Invaders, and Rogue terminal renders.
- **Multiplayer / Leaderboard Protocol**: Lightweight local and network high-score ledger.

## v2.0 — Community & AI Platform

*Local-first, AI-native platform for sharing, discovering, and co-authoring ASCII scenes.*

### Registry (Self-Hosted + Hosted)

| Component | Description |
|-----------|-------------|
| **`aa scene publish` / `install` / `search`** | CLI verbs mirroring cargo/npm; packages are `.aa` bundles (WASM + `scene.toml` + thumbnail) |
| **Hosted registry** | `registry.ascii-arcade.dev` — public, free, append-only; serves bundles + animated previews (MP4/GIF via headless capture) |
| **Self-hosted registry** | `aa registry serve` — single binary (Rust + SQLite), runs on any box; same API, federated discoverability via well-known URL |
| **Manifest (`scene.toml`)** | `name`, `version`, `author`, `wasm_hash`, `api_version`, `tags[]`, `capabilities{}`, `forked_from?`, `remixed_from?`, `ai_generated?` |
| **Preview pipeline** | Registry enqueues Playwright capture job on publish; stores 10s MP4 + 3s GIF + static PNG |

### AI Control API (Schema-Driven)

| Transport | Scope | Auth |
|-----------|-------|------|
| **WebSocket** (`/ws/control`) | Real-time control: `scene.switch`, `theme.set`, `session.start`, `session.pause`, `frame.capture` | Localhost: none. Hosted: API key (Bearer) with scopes `control:read`, `control:write` |
| **REST** (`/api/v1/`) | Scripting/automation: `GET /scenes`, `POST /sessions`, `PATCH /sessions/{id}`, `GET /sessions/{id}/frame` | Same as WS |
| **MCP Server** (`ascii-arcade-mcp`) | AI agent tools: `list_scenes`, `start_session`, `set_theme`, `capture_frame`, `search_registry` | Inherits local/hosted model; runs as stdio or HTTP+SSE |

> All verbs mirror the `aa` CLI — the API *is* the CLI, just networked. OpenAPI spec published at `/api/openapi.json`.

### Scene Sandboxing (Capability Model)

| Capability | Default | Manifest Key | Runtime Enforcement |
|------------|---------|--------------|---------------------|
| Network | `none` | `capabilities.network = "none \| fetch \| websocket"` | WASM imports only provide allowed host functions |
| Filesystem | `none` | `capabilities.fs = "none \| read:tmp \| write:tmp"` | Same — no host functions = no access |
| CPU budget | `16ms/frame` | `capabilities.max_frame_ms = 16` | Host measures frame time; kills scene if exceeded 3× consecutively |
| Memory | `64 MB` | `capabilities.max_memory_mb = 64` | WASM `memory` limit set at instantiation |

Registry **rejects** publish if `capabilities` exceeds baseline (`network=none, fs=none, max_frame_ms=16, max_memory_mb=64`) without manual review flag.

---

### Scripting & Automation Surface

```bash
# Declarative routines (DSL in .aa-script files)
aa script run morning.aa
#   at "07:00" scene=donut theme=ice
#   at "09:00" scene=matrix theme=hacker
#   on idle>300 scene=cycle

# Event hooks
aa hook on-scene-change "notify-send 'Now: {scene} ({theme})'"
aa hook on-session-end "aa scene export > ~/archive/{date}.aa"

# Cron-style scheduling
aa schedule "0 * * * *" "aa scene next"
aa schedule "@reboot" "aa run matrix --theme amber --autostart"
```

---

## v2.1 — AI Scene Generation

| Mode | Flow |
|------|------|
| **Procedural templates** | Parameterized generators in registry: `particle-field { density, gravity, palette }`, `text-wave { font, amplitude, speed }` — exposed as `aa scene generate particle-field --density 0.3` |
| **LLM co-authoring** | `aa scene generate "cyberpunk rain with glowing kanji"` → LLM produces `scene.toml` + Rust/WASM stub → user refines in browser editor → `aa scene publish` |
| **Remix workflow** | `aa scene fork matrix-rain --name my-matrix` → edit params → `aa scene publish` (manifest tracks `forked_from`) |
| **AI metadata** | Manifest field `ai_generated: { model, prompt, timestamp, human_edited: bool }` for transparency |

---

## v2.2 — Social Features

| Feature | Notes |
|---------|-------|
| **Follow / collections** | Users curate lists: "Best Matrix Variants", "Retro CRT Pack" |
| **Embedded gallery** | `aa web --gallery` serves a browsable scene explorer (reuses registry preview assets) |
| **Attribution graph** | Fork/remix chains visible on registry; "Original by @user, remixed by @you" |
| **Notifications** | WebSub / webhook on new versions of followed scenes |

---

## Out of Scope

- Proprietary closed game engines
- Heavy asset download requirements
- Centralized identity / social graph (federated, local-first only)
- Mobile apps (Android/iOS shells get registry + API in **future phases**)

---

For technical specifications, see [`docs/specs/`](docs/specs/).