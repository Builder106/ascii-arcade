# API plan

> Status: exploratory — no build started. This is a candidate list, not a commitment.

## Current state

`shells/web/src/main.rs`already exposes a small HTTP surface for its own bundled`xterm.js` frontend:

- `GET /` — static host page
- `GET /api/scenes` — list of available scene ids
- `GET /ws/{scene_id}` — WebSocket upgrade streaming rendered frames

This is sufficient for the web shell's own UI. Nothing here is a general-purpose public API yet, and nothing should become one without a concrete second consumer.

## Candidate use cases

Ranked by fit with the project as a single-user desktop tool, not a hosted service.

### Strong fits

1. **Remote control companion app.** iOS (and the planned Android shell) currently render their own copy of the scenes locally. A control API (`POST /api/scene`, `/api/theme`, `/api/speed`) would let the phone act as a remote for the *desktop* wallpaper instance instead of duplicating rendering — flip scenes/themes on the Mac/PC from the phone.
2. **Local automation/scripting hook.** The `aa`CLI already exposes`play`/`run`/`web`/`autostart`/`scenes`/`themes`. Exposing the same verbs over a local HTTP API lets external tools trigger them: a Raycast/Alfred command, a Stream Deck button, a cron job that swaps to the Ghost theme at sunset, a Home Assistant webhook tied to time-of-day or focus mode.

### Plausible, needs a real trigger first

1. **Embeddable widget.** The existing WebSocket + xterm.js pipeline is close to a drop-in live demo widget — a `<script>` embed or iframe pointed at a hosted instance so people can put a live donut/matrix-rain animation on a personal site or README.
2. **Streaming overlay control.** An OBS browser source pointed at the web shell's WebSocket, with the API as the control channel — a Twitch bot could POST a scene change on a sub/cheer event.

### Bigger lift, not worth pursuing first

1. **Scene/theme marketplace.** User-submitted scenes discovered via API. Needs a submission/review pipeline before the API question even matters.
2. **Multi-instance orchestration.** Push one scene/theme to several machines or monitors at once. Niche unless there's an actual multi-monitor/multi-machine setup driving it.
3. **Programmatic asset export.** `POST /api/export?scene=donut&duration=3s` to trigger the same Live Photo/video-render pipeline the iOS exporter already has (`shells/ios/AsciiArcade/Export/`), for generating README GIFs/demo clips on demand instead of manually recording.

## Recommendation

Don't build a general-purpose API speculatively. #1 and #2 are the ones that fit a single-user desktop tool without inventing new audiences — they turn the API into a control surface for the owner, not a public service. #3 and #4 only make sense once there's an actual reason for someone else to hit a hosted instance. Revisit this doc once one of these has a concrete trigger (e.g. "I want to change the wallpaper from my phone" or "I want to embed this on my site").
