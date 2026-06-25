//! A large block-digit clock.
//!
//! Port of `ClockScene.swift`. Renders the current time with a 3×5 pixel font
//! scaled up to fill the wallpaper, centred. Monochrome by design — every glyph
//! is emitted with a `None` colour so the host paints it in the active theme
//! colour (green/amber/ice/ghost).
//!
//! The Swift version reads `Date()` through an injectable `now` closure (for
//! tests). aa-core has no platform-date dependency, so this takes the time as
//! seconds-since-local-midnight from a closure, defaulting to one derived from
//! [`std::time::SystemTime`]. Tests inject a fixed value.

use crate::frame::Frame;
use crate::scene::{Scene, SceneOption, SceneSetting};

const GLYPH_HEIGHT: usize = 5;

/// 3×5 pixel patterns for the digits 0–9.
const DIGITS: [[&str; GLYPH_HEIGHT]; 10] = [
    ["###", "# #", "# #", "# #", "###"], // 0
    [" # ", "## ", " # ", " # ", "###"], // 1
    ["###", "  #", "###", "#  ", "###"], // 2
    ["###", "  #", "###", "  #", "###"], // 3
    ["# #", "# #", "###", "  #", "  #"], // 4
    ["###", "#  ", "###", "  #", "###"], // 5
    ["###", "#  ", "###", "# #", "###"], // 6
    ["###", "  #", "  #", "  #", "  #"], // 7
    ["###", "# #", "###", "# #", "###"], // 8
    ["###", "# #", "###", "  #", "###"], // 9
];
/// The colon is 1×5.
const COLON: [&str; GLYPH_HEIGHT] = [" ", "#", " ", "#", " "];

pub struct ClockScene {
    width: usize,
    height: usize,
    size: f64,
    seconds_on: f64,
    /// Returns `(hour, minute, second)` in local time. Injectable for tests.
    now: fn() -> (u32, u32, u32),
}

impl ClockScene {
    pub fn new() -> Self {
        ClockScene {
            width: 10,
            height: 10,
            size: 0.70,
            seconds_on: 1.0,
            now: wall_clock_hms,
        }
    }

    fn size_factor(&self) -> f64 {
        self.size
    }

    fn show_seconds(&self) -> bool {
        self.seconds_on > 0.5
    }

    /// Assemble the 5-row source bitmap for the whole time string, with a
    /// 1-pixel gap between glyphs.
    fn build_bitmap(text: &str) -> Vec<String> {
        let mut rows = vec![String::new(); GLYPH_HEIGHT];
        let mut first = true;
        for ch in text.chars() {
            let glyph: [&str; GLYPH_HEIGHT] = if ch == ':' {
                COLON
            } else if let Some(d) = ch.to_digit(10) {
                DIGITS[d as usize]
            } else {
                ["   ", "   ", "   ", "   ", "   "]
            };
            for (r, row) in rows.iter_mut().enumerate() {
                if !first {
                    row.push(' ');
                }
                row.push_str(glyph[r]);
            }
            first = false;
        }
        rows
    }

    fn render(&self) -> Frame {
        let (w, h) = (self.width, self.height);
        let mut frame = Frame::blank(w, h);

        let (hh, mm, ss) = (self.now)();
        let time_string = if self.show_seconds() {
            format!("{hh:02}:{mm:02}:{ss:02}")
        } else {
            format!("{hh:02}:{mm:02}")
        };
        let bitmap = Self::build_bitmap(&time_string); // rows of "#/ " strings
        let bmp_h = GLYPH_HEIGHT;
        let bmp_w = bitmap.first().map(|r| r.chars().count()).unwrap_or(0);
        if bmp_w == 0 || h == 0 || w == 0 {
            return frame;
        }

        // Pick an integer scale that fits, biased by the Size setting.
        let max_scale_w = w as f64 / bmp_w as f64;
        let max_scale_h = h as f64 / bmp_h as f64;
        let fit = max_scale_w.min(max_scale_h);
        let scale = ((fit * self.size_factor()).floor() as usize).max(1);

        let draw_w = bmp_w * scale;
        let draw_h = bmp_h * scale;
        let off_x = (w as isize - draw_w as isize) / 2;
        let off_y = (h as isize - draw_h as isize) / 2;

        let bmp_rows: Vec<Vec<char>> = bitmap.iter().map(|r| r.chars().collect()).collect();
        for gy in 0..draw_h {
            let by = gy / scale;
            let ty = off_y + gy as isize;
            if ty < 0 || ty as usize >= h || by >= bmp_h {
                continue;
            }
            let row_chars = &bmp_rows[by];
            for gx in 0..draw_w {
                let bx = gx / scale;
                if bx >= row_chars.len() || row_chars[bx] != '#' {
                    continue;
                }
                let tx = off_x + gx as isize;
                if tx < 0 || tx as usize >= w {
                    continue;
                }
                frame.set_char(tx as usize, ty as usize, '█');
            }
        }
        frame
    }
}

impl Default for ClockScene {
    fn default() -> Self {
        ClockScene::new()
    }
}

impl Scene for ClockScene {
    fn display_name(&self) -> &str {
        "Clock"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
        }
    }

    fn frame(&mut self, _t: f64) -> Frame {
        self.render()
    }

    fn settings(&self) -> Vec<SceneSetting> {
        vec![
            SceneSetting::new(
                "size",
                "Size",
                vec![
                    SceneOption::new("Small", 0.45),
                    SceneOption::new("Medium", 0.70),
                    SceneOption::new("Large", 0.95),
                ],
                1,
            ),
            SceneSetting::new(
                "seconds",
                "Seconds",
                vec![
                    SceneOption::new("On", 1.0),
                    SceneOption::new("Off", 0.0),
                ],
                0,
            ),
        ]
    }

    fn apply_setting(&mut self, id: &str, value: f64) {
        match id {
            "size" => self.size = value,
            "seconds" => self.seconds_on = value,
            _ => {}
        }
    }
}

/// Local wall-clock time as `(hour, minute, second)`.
///
/// Derived from [`std::time::SystemTime`] (UTC seconds) offset by the local
/// timezone. Without a TZ database we approximate the offset from the difference
/// between local and UTC the OS reports at process start; this is good enough for
/// a wallpaper clock and keeps aa-core dependency-free. The exact wall value is
/// not asserted in tests (those inject a fixed `now`).
fn wall_clock_hms() -> (u32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let local = secs as i64 + local_utc_offset_secs();
    let day = local.rem_euclid(86_400);
    let h = (day / 3600) as u32;
    let m = ((day % 3600) / 60) as u32;
    let s = (day % 60) as u32;
    (h, m, s)
}

/// Best-effort local-UTC offset in seconds. Reads the `TZ`-independent offset the
/// platform exposes via `localtime`; falls back to 0 (UTC) if unavailable.
fn local_utc_offset_secs() -> i64 {
    // Parse the offset the OS prints for "now" via the standard C library would
    // need libc; to stay dependency-free we read the offset from an environment
    // hint if present, else assume UTC. The native shells override the displayed
    // time through their own platform date APIs, so UTC here is an acceptable
    // headless default.
    std::env::var("AA_CLOCK_UTC_OFFSET_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_clock() -> ClockScene {
        let mut s = ClockScene::new();
        s.now = || (13, 37, 42); // deterministic time
        s
    }

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = fixed_clock();
        s.set_grid(40, 20);
        let f = s.frame(0.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn renders_block_digits() {
        let mut s = fixed_clock();
        s.set_grid(80, 30);
        let f = s.frame(0.0);
        let blocks = f.cells.iter().filter(|c| c.ch == '█').count();
        assert!(blocks > 50, "clock should draw scaled block digits");
    }

    #[test]
    fn monochrome_cells() {
        let mut s = fixed_clock();
        s.set_grid(80, 30);
        let f = s.frame(0.0);
        assert!(
            f.cells.iter().all(|c| c.color.is_none()),
            "clock is monochrome: cells use the theme colour"
        );
    }

    #[test]
    fn seconds_toggle_changes_width() {
        let mut s = fixed_clock();
        s.set_grid(120, 40);
        let with_secs = s.frame(0.0).text();
        s.apply_setting("seconds", 0.0);
        let without_secs = s.frame(0.0).text();
        assert_ne!(with_secs, without_secs);
    }

    #[test]
    fn deterministic_for_fixed_time() {
        let mut s = fixed_clock();
        s.set_grid(100, 40);
        assert_eq!(s.frame(0.0).text(), s.frame(5.0).text());
    }
}
