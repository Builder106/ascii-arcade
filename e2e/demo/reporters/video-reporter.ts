import type { Reporter, TestCase, TestResult } from "@playwright/test/reporter";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, renameSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";

// Collects each test's video in onTestEnd, then (deferred to onEnd, once Playwright
// has flushed the files) slugifies, drops warmups + 0-byte clips, and transcodes
// webm -> mp4 into e2e/recordings/.
export default class VideoReporter implements Reporter {
  private pending: { sourcePath: string; slug: string }[] = [];
  private readonly outDir = join(process.cwd(), "recordings");

  private slugify(title: string): string {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "");
  }

  onTestEnd(test: TestCase, result: TestResult) {
    const video = result.attachments.find((a) => a.name === "video");
    if (!video?.path) return;
    // Feature title + scenario title → stable filename.
    const titles = test.titlePath().filter(Boolean);
    const slug = this.slugify(titles.slice(-2).join("-"));
    this.pending.push({ sourcePath: video.path, slug });
  }

  async onEnd() {
    mkdirSync(this.outDir, { recursive: true });
    for (const { sourcePath, slug } of this.pending) {
      if (slug.startsWith("warmup") || slug.startsWith("00-warmup")) {
        this.cleanup(sourcePath);
        continue;
      }
      if (!existsSync(sourcePath) || statSync(sourcePath).size === 0) {
        this.cleanup(sourcePath);
        continue;
      }
      const mp4 = join(this.outDir, `${slug}.mp4`);
      try {
        execFileSync(
          "ffmpeg",
          ["-y", "-i", sourcePath, "-c:v", "libx264", "-preset", "veryfast",
           "-pix_fmt", "yuv420p", "-movflags", "+faststart", mp4],
          { stdio: "ignore" }
        );
        console.log(`🎞  wrote ${mp4}`);
        this.cleanup(sourcePath);
      } catch (e) {
        console.warn(`ffmpeg failed for ${slug}:`, (e as Error).message);
      }
    }
  }

  private cleanup(sourcePath: string) {
    try {
      rmSync(sourcePath, { force: true });
      const dir = dirname(sourcePath);
      rmSync(dir, { recursive: true, force: true });
    } catch {
      /* best-effort */
    }
  }
}
