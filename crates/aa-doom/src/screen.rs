//! Reconstructs `doom_ascii`'s ANSI terminal stream into a fixed character grid
//! with per-cell truecolor.
//!
//! Ported from `DoomScreenBuffer.swift`. `doom_ascii -chars block` redraws every
//! frame as: cursor-home (`ESC[;H`), an optional full clear (`ESC[2J`), a bold
//! flag (`ESC[1m`, dropped), then per-pixel truecolor SGR codes
//! (`ESC[38;2;R;G;Bm`) each followed by a block glyph, ending with a reset
//! (`ESC[0m`). It also emits an OSC window-title sequence (`ESC]2;…ESC\`).
//!
//! ## Colour decision
//! The current Swift renderer paints DOOM as a **colour bitmap** (git:
//! "render DOOM at native resolution as a colour bitmap"), and `DoomScreenBuffer`
//! tracks the `38;2;R;G;B` foreground per cell via `colorGrid` /
//! `coloredSnapshot()`. So this port **parses SGR truecolor into per-cell
//! [`RgbColor`]** and stores it on each [`Cell`] — `DoomScene::frame` returns a
//! fully-coloured [`Frame`]. A cell with no colour set stays `None`, meaning
//! "paint in the active theme colour" (matching the Swift `nil` convention).
//!
//! Bytes arrive in arbitrary chunks off the PTY reader thread, so escape
//! sequences and multibyte UTF-8 glyphs can straddle a chunk boundary; partial
//! tails are stashed in `pending` and resumed on the next `feed`.

use aa_core::{Cell, Frame, RgbColor};

/// A mutable terminal grid driven by a stream of `doom_ascii` ANSI bytes.
pub struct ScreenBuffer {
    width: usize,
    height: usize,
    /// `width * height` cells, row-major. A cell's `color` is the truecolor the
    /// most recent SGR set (or `None` after a reset).
    cells: Vec<Cell>,
    current_color: Option<RgbColor>,
    cursor_row: usize,
    cursor_col: usize,
    pending: Vec<u8>,
}

impl ScreenBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        ScreenBuffer {
            width,
            height,
            cells: vec![Cell::BLANK; width * height],
            current_color: None,
            cursor_row: 0,
            cursor_col: 0,
            pending: Vec::new(),
        }
    }

    /// Snapshot the grid as an owned [`Frame`].
    pub fn snapshot(&self) -> Frame {
        Frame {
            width: self.width,
            height: self.height,
            cells: self.cells.clone(),
        }
    }

    /// Reset the grid to all-blank and home the cursor.
    pub fn clear(&mut self) {
        for c in &mut self.cells {
            *c = Cell::BLANK;
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Centre an informational line in an otherwise-blank grid (e.g. an error
    /// when the binary is missing). Ported from `showMessage`.
    pub fn show_message(&mut self, message: &str) {
        self.clear();
        let row = self.height / 2;
        let chars: Vec<char> = message.chars().collect();
        let start = self.width.saturating_sub(chars.len()) / 2;
        for (i, ch) in chars.iter().enumerate() {
            let col = start + i;
            if col < self.width {
                let idx = row * self.width + col;
                self.cells[idx] = Cell::new(*ch, None);
            }
        }
    }

    /// Feed raw PTY bytes, advancing the parser. Incomplete escape sequences and
    /// multibyte glyphs that straddle a chunk boundary are stashed and resumed.
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut buf = std::mem::take(&mut self.pending);
        buf.extend_from_slice(bytes);

        let count = buf.len();
        let mut i = 0;
        while i < count {
            let b = buf[i];
            match b {
                0x1b => {
                    // ESC — need at least the next byte to classify.
                    if i + 1 >= count {
                        self.pending = buf[i..].to_vec();
                        return;
                    }
                    let n = buf[i + 1];
                    if n == b'[' {
                        // CSI: ESC [ params <final 0x40..=0x7e>
                        match scan_csi_terminator(&buf, i + 2) {
                            Some(end) => {
                                self.apply_csi(&buf[i + 2..end], buf[end]);
                                i = end + 1;
                            }
                            None => {
                                self.pending = buf[i..].to_vec();
                                return;
                            }
                        }
                    } else if n == b']' {
                        // OSC: ESC ] ... (BEL | ESC \)
                        match scan_osc_terminator(&buf, i + 2) {
                            Some(end) => i = end,
                            None => {
                                self.pending = buf[i..].to_vec();
                                return;
                            }
                        }
                    } else {
                        // Two-byte escape (e.g. the ST `ESC \`). Consume both.
                        i += 2;
                    }
                }
                0x0a => {
                    // LF — treat as CR+LF (doom relies on EOL conversion).
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                    i += 1;
                }
                0x0d => {
                    self.cursor_col = 0;
                    i += 1;
                }
                0x00..=0x1f => {
                    // Other control bytes — ignore.
                    i += 1;
                }
                _ => {
                    if b < 0x80 {
                        self.place(b as char);
                        i += 1;
                    } else {
                        let len = utf8_length(b);
                        if i + len > count {
                            self.pending = buf[i..].to_vec();
                            return;
                        }
                        match std::str::from_utf8(&buf[i..i + len]) {
                            Ok(s) => {
                                if let Some(ch) = s.chars().next() {
                                    self.place(ch);
                                }
                            }
                            Err(_) => { /* invalid sequence: skip it */ }
                        }
                        i += len;
                    }
                }
            }
        }
    }

    fn place(&mut self, ch: char) {
        if self.cursor_row >= self.height {
            self.cursor_col += 1;
            return;
        }
        if self.cursor_col < self.width {
            let idx = self.cursor_row * self.width + self.cursor_col;
            self.cells[idx] = Cell::new(ch, self.current_color);
        }
        self.cursor_col += 1;
    }

    fn apply_csi(&mut self, params: &[u8], final_byte: u8) {
        match final_byte {
            b'H' | b'f' => {
                // Cursor position (1-based, default 1;1).
                let s = String::from_utf8_lossy(params);
                let mut parts = s.split(';');
                let row1 = parts.next().and_then(|p| p.parse::<usize>().ok()).unwrap_or(1);
                let col1 = parts.next().and_then(|p| p.parse::<usize>().ok()).unwrap_or(1);
                self.cursor_row = row1.saturating_sub(1).min(self.height - 1);
                self.cursor_col = col1.saturating_sub(1).min(self.width - 1);
            }
            b'J' => {
                // Erase display (any mode → full clear, as doom only emits 2J).
                for c in &mut self.cells {
                    *c = Cell::BLANK;
                }
            }
            b'K' => {
                let mode = String::from_utf8_lossy(params).parse::<usize>().unwrap_or(0);
                self.erase_line(mode);
            }
            b'm' => self.apply_sgr(params),
            _ => {}
        }
    }

    /// Interpret an SGR parameter list. We track only the foreground colour:
    /// empty / `0` / `39` reset it; `38;2;R;G;B` sets a truecolor value (what
    /// `doom_ascii -chars block` emits). Bold (`1`) and everything else is
    /// dropped.
    fn apply_sgr(&mut self, params: &[u8]) {
        let s = String::from_utf8_lossy(params);
        // Each `;`-separated field; an empty field parses to None and resets.
        let parts: Vec<Option<i64>> = s.split(';').map(|p| p.parse::<i64>().ok()).collect();

        if parts.is_empty() || (parts.len() == 1 && matches!(parts[0], Some(0) | None)) {
            self.current_color = None;
            return;
        }

        let mut i = 0;
        while i < parts.len() {
            match parts[i] {
                Some(0) | Some(39) => {
                    self.current_color = None;
                    i += 1;
                }
                Some(38) => {
                    if i + 4 < parts.len() && parts[i + 1] == Some(2) {
                        if let (Some(r), Some(g), Some(b)) =
                            (parts[i + 2], parts[i + 3], parts[i + 4])
                        {
                            self.current_color =
                                Some(RgbColor::new(clamp_byte(r), clamp_byte(g), clamp_byte(b)));
                            i += 5;
                            continue;
                        }
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    fn erase_line(&mut self, mode: usize) {
        if self.cursor_row >= self.height {
            return;
        }
        let base = self.cursor_row * self.width;
        match mode {
            1 => {
                // Start of line to cursor (inclusive).
                let end = self.cursor_col.min(self.width - 1);
                for c in 0..=end {
                    self.cells[base + c] = Cell::BLANK;
                }
            }
            2 => {
                for c in 0..self.width {
                    self.cells[base + c] = Cell::BLANK;
                }
            }
            _ => {
                // Cursor to end of line.
                if self.cursor_col < self.width {
                    for c in self.cursor_col..self.width {
                        self.cells[base + c] = Cell::BLANK;
                    }
                }
            }
        }
    }
}

fn clamp_byte(v: i64) -> u8 {
    v.clamp(0, 255) as u8
}

/// Find the index of a CSI final byte (`0x40..=0x7e`) at or after `start`.
fn scan_csi_terminator(buf: &[u8], start: usize) -> Option<usize> {
    (start..buf.len()).find(|&j| (0x40..=0x7e).contains(&buf[j]))
}

/// Find the end index (one past the terminator) of an OSC sequence, which ends
/// at BEL (`0x07`) or ST (`ESC \`).
fn scan_osc_terminator(buf: &[u8], start: usize) -> Option<usize> {
    let count = buf.len();
    let mut j = start;
    while j < count {
        if buf[j] == 0x07 {
            return Some(j + 1); // BEL
        }
        if buf[j] == 0x1b {
            if j + 1 < count {
                if buf[j + 1] == 0x5c {
                    return Some(j + 2); // ST = ESC \
                }
            } else {
                return None; // possible partial ST
            }
        }
        j += 1;
    }
    None
}

fn utf8_length(lead: u8) -> usize {
    if lead & 0xE0 == 0xC0 {
        2
    } else if lead & 0xF0 == 0xE0 {
        3
    } else if lead & 0xF8 == 0xF0 {
        4
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_text(buf: &ScreenBuffer) -> String {
        buf.snapshot().text()
    }

    #[test]
    fn plain_text_fills_grid_from_home() {
        let mut buf = ScreenBuffer::new(3, 2);
        buf.feed(b"\x1b[;Hab\ncd");
        assert_eq!(frame_text(&buf), "ab \ncd ");
    }

    #[test]
    fn cursor_position_is_one_based() {
        let mut buf = ScreenBuffer::new(4, 3);
        // Row 2, col 3 (1-based) → grid (row 1, col 2).
        buf.feed(b"\x1b[2;3HX");
        let f = buf.snapshot();
        assert_eq!(f.cells[f.idx(2, 1)].ch, 'X');
    }

    #[test]
    fn erase_display_clears_everything() {
        let mut buf = ScreenBuffer::new(3, 2);
        buf.feed(b"\x1b[;Habcdef");
        buf.feed(b"\x1b[2J");
        assert_eq!(frame_text(&buf), "   \n   ");
    }

    #[test]
    fn truecolor_sgr_sets_per_cell_color() {
        let mut buf = ScreenBuffer::new(2, 1);
        // Red 'X', then reset, then plain 'Y' (theme colour).
        buf.feed(b"\x1b[;H\x1b[38;2;255;0;0mX\x1b[0mY");
        let f = buf.snapshot();
        assert_eq!(f.cells[0], Cell::new('X', Some(RgbColor::new(255, 0, 0))));
        assert_eq!(f.cells[1], Cell::new('Y', None));
    }

    #[test]
    fn sgr_39_resets_foreground() {
        let mut buf = ScreenBuffer::new(2, 1);
        buf.feed(b"\x1b[;H\x1b[38;2;1;2;3mA\x1b[39mB");
        let f = buf.snapshot();
        assert_eq!(f.cells[0].color, Some(RgbColor::new(1, 2, 3)));
        assert_eq!(f.cells[1].color, None);
    }

    #[test]
    fn bold_and_other_sgr_are_dropped_but_keep_color() {
        let mut buf = ScreenBuffer::new(1, 1);
        // Bold flag then a colour in the same run; bold must not clobber colour.
        buf.feed(b"\x1b[;H\x1b[1m\x1b[38;2;10;20;30mZ");
        let f = buf.snapshot();
        assert_eq!(f.cells[0], Cell::new('Z', Some(RgbColor::new(10, 20, 30))));
    }

    #[test]
    fn sgr_value_clamped_to_byte() {
        let mut buf = ScreenBuffer::new(1, 1);
        buf.feed(b"\x1b[;H\x1b[38;2;999;0;0mZ");
        let f = buf.snapshot();
        assert_eq!(f.cells[0].color, Some(RgbColor::new(255, 0, 0)));
    }

    #[test]
    fn osc_window_title_is_skipped() {
        let mut buf = ScreenBuffer::new(5, 1);
        // OSC set-title (ESC ] 2 ; doom ESC \) then real content.
        buf.feed(b"\x1b[;H\x1b]2;doom\x1b\\hi");
        assert_eq!(frame_text(&buf), "hi   ");
    }

    #[test]
    fn osc_terminated_by_bel_is_skipped() {
        let mut buf = ScreenBuffer::new(3, 1);
        buf.feed(b"\x1b[;H\x1b]2;t\x07ab");
        assert_eq!(frame_text(&buf), "ab ");
    }

    #[test]
    fn split_escape_across_feeds_is_resumed() {
        let mut buf = ScreenBuffer::new(2, 2);
        // Cursor-home split mid-CSI, then content split mid-SGR.
        buf.feed(b"\x1b[");
        buf.feed(b";HA");
        buf.feed(b"\x1b[38;2;1;");
        buf.feed(b"2;3mB");
        let f = buf.snapshot();
        assert_eq!(f.cells[0], Cell::new('A', None));
        assert_eq!(f.cells[1], Cell::new('B', Some(RgbColor::new(1, 2, 3))));
    }

    #[test]
    fn multibyte_glyph_split_across_feeds() {
        let mut buf = ScreenBuffer::new(2, 1);
        // '█' (U+2588) is 3 UTF-8 bytes: E2 96 88. Split after the first byte.
        buf.feed(b"\x1b[;H\xe2");
        buf.feed(b"\x96\x88X");
        let f = buf.snapshot();
        assert_eq!(f.cells[0].ch, '█');
        assert_eq!(f.cells[1].ch, 'X');
    }

    #[test]
    fn out_of_bounds_cursor_does_not_panic() {
        let mut buf = ScreenBuffer::new(2, 2);
        // Write past the grid; rows beyond height are dropped, no panic.
        buf.feed(b"\x1b[;HABCDEFGHIJ");
        // First row filled, overflow ignored.
        assert_eq!(frame_text(&buf), "AB\nCD");
    }

    #[test]
    fn show_message_centers_text() {
        let mut buf = ScreenBuffer::new(10, 3);
        buf.show_message("hi");
        let f = buf.snapshot();
        // Centered on the middle row.
        let row = 3 / 2;
        let text: String = (0..10).map(|c| f.cells[row * 10 + c].ch).collect();
        assert_eq!(text.trim(), "hi");
    }

    #[test]
    fn erase_line_to_end_clears_tail() {
        let mut buf = ScreenBuffer::new(5, 1);
        buf.feed(b"\x1b[;HABCDE");
        // Home, move to col 3 (1-based), erase to end of line.
        buf.feed(b"\x1b[1;3H\x1b[0K");
        assert_eq!(frame_text(&buf), "AB   ");
    }

    #[test]
    fn realistic_doom_frame_fragment() {
        // A miniature of an actual doom_ascii frame: home + clear + bold, two
        // coloured block pixels, reset.
        let mut buf = ScreenBuffer::new(4, 1);
        buf.feed(
            b"\x1b[;H\x1b[2J\x1b[1m\
              \x1b[38;2;200;0;0m\xe2\x96\x88\
              \x1b[38;2;0;0;200m\xe2\x96\x88\
              \x1b[0m",
        );
        let f = buf.snapshot();
        assert_eq!(f.cells[0], Cell::new('█', Some(RgbColor::new(200, 0, 0))));
        assert_eq!(f.cells[1], Cell::new('█', Some(RgbColor::new(0, 0, 200))));
        assert_eq!(f.cells[2], Cell::BLANK);
    }
}
