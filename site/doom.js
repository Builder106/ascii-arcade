/*
 * DOOM as a frame source, which is how the app already treats it: one contract,
 * two implementations. RecordedDoom replays captured attract-mode frames today
 * and a WasmDoom can take its place without touching this page.
 *
 * The frames are text captured from the real doom_ascii binary, so nothing GPL
 * is redistributed here. Only the characters it drew.
 */

const HIGH_DENSITY = new Set(["@", "M", "W", "$", "%", "#", "0", "Q", "O", "X", "B", "R"]);
const MID_DENSITY = new Set(["+", "*", "=", "-", "i", "l", "t", "r", "f", "u", "v", "j", "z"]);

function escapeHtml(ch) {
  if (ch === "&") return "&amp;";
  if (ch === "<") return "&lt;";
  if (ch === ">") return "&gt;";
  return ch;
}

export function colorizeFrame(frameData, palette = []) {
  if (typeof frameData === "string") {
    if (frameData.includes("<span")) return frameData;
    let html = "";
    for (let i = 0; i < frameData.length; i++) {
      const ch = frameData[i];
      if (ch === "\n") html += "\n";
      else if (ch === " ") html += " ";
      else {
        const safe = escapeHtml(ch);
        if (HIGH_DENSITY.has(ch)) html += `<span class="d-hot">${safe}</span>`;
        else if (MID_DENSITY.has(ch)) html += `<span class="d-mid">${safe}</span>`;
        else html += `<span class="d-dim">${safe}</span>`;
      }
    }
    return html;
  }

  if (!Array.isArray(frameData)) return "";

  let html = "";
  for (let i = 0; i < frameData.length; i++) {
    const item = frameData[i];
    if (!Array.isArray(item)) continue;
    const [topIdx, botIdx, count] = item;

    if (topIdx === -1 && botIdx === -1) {
      html += "\n";
    } else {
      const topColor = palette[topIdx];
      const botColor = palette[botIdx];

      if (topColor && botColor) {
        if (topColor === botColor) {
          const blocks = "█".repeat(count);
          html += `<span style="color:${topColor}">${blocks}</span>`;
        } else {
          const blocks = "▀".repeat(count);
          html += `<span style="color:${topColor};background:${botColor}">${blocks}</span>`;
        }
      } else if (topColor) {
        const blocks = "▀".repeat(count);
        html += `<span style="color:${topColor}">${blocks}</span>`;
      } else if (botColor) {
        const blocks = "▄".repeat(count);
        html += `<span style="color:${botColor}">${blocks}</span>`;
      } else {
        html += " ".repeat(count);
      }
    }
  }
  return html;
}

export class RecordedDoom {
  constructor(data) {
    this.frames = data.frames ?? [];
    this.palette = data.palette ?? [];
    this.fps = data.fps ?? 8;
    this.startedAt = 0;
    this.running = false;
  }

  start() {
    this.startedAt = performance.now();
    this.running = true;
  }

  stop() {
    this.running = false;
  }

  frame() {
    if (!this.running || this.frames.length === 0) return null;
    const elapsed = (performance.now() - this.startedAt) / 1000;
    const idx = Math.floor(elapsed * this.fps) % this.frames.length;
    return { frame: this.frames[idx], palette: this.palette };
  }
}

export async function mountDoom(preEl, buttonEl) {
  let source = null;

  try {
    const res = await fetch("assets/doom-attract.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    source = new RecordedDoom(await res.json());
    source.start();
  } catch (err) {
    console.warn("DOOM recording unavailable", err);
  }

  if (source) {
    let last = null;
    const draw = () => {
      if (document.visibilityState === "visible") {
        const item = source.frame();
        if (item && item.frame !== last) {
          preEl.innerHTML = colorizeFrame(item.frame, item.palette);
          last = item.frame;
        }
      }
      requestAnimationFrame(draw);
    };
    requestAnimationFrame(draw);
  }

  // Somebody clicked this on purpose, so silence is the wrong answer.
  buttonEl.addEventListener("click", () => {
    let status = document.getElementById("doomStatus");
    if (!status) {
      status = document.createElement("p");
      status.id = "doomStatus";
      status.className = "open__note";
      status.setAttribute("role", "status");
      buttonEl.closest(".open__acts").after(status);
    }
    status.textContent =
      "Not playable in the browser yet: DOOM needs a GPL binary and a real terminal. Clone the repo and run ./scripts/setup.sh && swift run AsciiArcade.";
  });
}
