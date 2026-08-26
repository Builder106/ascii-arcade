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
        self.cells.fill(Cell::BLANK);
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
                let mut parts = params.split(|&b| b == b';');
                let row1 = parts.next().and_then(parse_uint_field).unwrap_or(1);
                let col1 = parts.next().and_then(parse_uint_field).unwrap_or(1);
                self.cursor_row = row1.saturating_sub(1).min(self.height - 1);
                self.cursor_col = col1.saturating_sub(1).min(self.width - 1);
            }
            b'J' => {
                // Erase display (any mode → full clear, as doom only emits 2J).
                self.cells.fill(Cell::BLANK);
            }
            b'K' => {
                let mode = parse_uint_field(params).unwrap_or(0);
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
    ///
    /// DOOM emits one of these per colored pixel, so this parses the raw
    /// `;`-separated ASCII digit fields directly off `params` — no UTF-8
    /// validation or heap allocation, since SGR params are always plain digits.
    fn apply_sgr(&mut self, params: &[u8]) {
        if params.is_empty() {
            self.current_color = None;
            return;
        }
        let mut fields = params.split(|&b| b == b';');
        while let Some(field) = fields.next() {
            match parse_int_field(field) {
                Some(0) | Some(39) => self.current_color = None,
                Some(38) => {
                    // Only a well-formed `38;2;R;G;B` (as doom always emits) sets
                    // colour; anything shorter is dropped along with the fields
                    // already consumed peeking ahead for it.
                    let mode = fields.next().and_then(parse_int_field);
                    let r = fields.next().and_then(parse_int_field);
                    let g = fields.next().and_then(parse_int_field);
                    let b = fields.next().and_then(parse_int_field);
                    if mode == Some(2) {
                        if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                            self.current_color =
                                Some(RgbColor::new(clamp_byte(r), clamp_byte(g), clamp_byte(b)));
                        }
                    }
                }
                _ => {}
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

/// Parse a single ANSI parameter field (plain ASCII digits, no allocation).
/// Empty or non-digit input yields `None`, matching how `"".parse::<i64>()`
/// and a malformed field would behave.
fn parse_int_field(field: &[u8]) -> Option<i64> {
    if field.is_empty() {
        return None;
    }
    let mut v: i64 = 0;
    for &b in field {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (b - b'0') as i64;
    }
    Some(v)
}

fn parse_uint_field(field: &[u8]) -> Option<usize> {
    parse_int_field(field).and_then(|v| usize::try_from(v).ok())
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
        // Write past the end of a row without a newline. We deliberately do NOT
        // auto-wrap: doom_ascii emits an explicit newline per scanline, so
        // wrapping would double-advance and drop a row. Overflow chars past the
        // row are discarded; the key assertion is simply that this never panics.
        buf.feed(b"\x1b[;HABCDEFGHIJ");
        assert_eq!(frame_text(&buf), "AB\n  ");
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

    #[test]
    fn erase_line_mode_1_and_mode_2() {
        let mut buf = ScreenBuffer::new(5, 2);
        buf.feed(b"\x1b[;HABCDE\r\nFGHIJ");
        // Mode 1: start of line to cursor (inclusive). Put cursor at col 3 (1-based), row 1.
        buf.feed(b"\x1b[1;3H\x1b[1K");
        let f = buf.snapshot();
        // Row 0 cols 0..=2 should be blank, rest untouched
        assert_eq!(
            &f.cells[0..5].iter().map(|c| c.ch).collect::<String>(),
            "   DE"
        );

        // Mode 2: entire line. Put cursor at row 2, erase entire line.
        buf.feed(b"\x1b[2;1H\x1b[2K");
        let f2 = buf.snapshot();
        assert_eq!(
            &f2.cells[5..10].iter().map(|c| c.ch).collect::<String>(),
            "     "
        );
    }

    #[test]
    fn carriage_return_and_control_characters() {
        let mut buf = ScreenBuffer::new(5, 1);
        // CR resets cursor_col to 0
        buf.feed(b"ABC\rZ");
        assert_eq!(frame_text(&buf), "ZBC  ");

        // Other control bytes (0x00..=0x1f like 0x01, 0x08) are ignored
        buf.feed(b"\x1b[;H\x01\x02Hello");
        assert_eq!(frame_text(&buf), "Hello");
    }

    #[test]
    fn csi_cursor_position_f_and_default_coords() {
        let mut buf = ScreenBuffer::new(4, 3);
        // Using 'f' instead of 'H'
        buf.feed(b"\x1b[2;2fQ");
        let f = buf.snapshot();
        assert_eq!(f.cells[f.idx(1, 1)].ch, 'Q');

        // Defaults to 1;1 when params are empty or missing
        buf.feed(b"\x1b[fW");
        let f2 = buf.snapshot();
        assert_eq!(f2.cells[f2.idx(0, 0)].ch, 'W');
    }

    #[test]
    fn malformed_sgr_and_empty_sgr_resets_color() {
        let mut buf = ScreenBuffer::new(3, 1);
        // Empty SGR \x1b[m resets color
        buf.feed(b"\x1b[38;2;100;100;100mA\x1b[mB");
        let f = buf.snapshot();
        assert_eq!(f.cells[0].color, Some(RgbColor::new(100, 100, 100)));
        assert_eq!(f.cells[1].color, None);

        // Malformed 38 without enough components is ignored
        buf.feed(b"\x1b[38;5;123mC");
        let f2 = buf.snapshot();
        assert_eq!(f2.cells[2].ch, 'C');
    }

    #[test]
    fn clear_and_show_message_with_overflow() {
        let mut buf = ScreenBuffer::new(4, 2);
        buf.feed(b"ABCD");
        buf.clear();
        assert_eq!(frame_text(&buf), "    \n    ");

        // Message longer than buffer width truncates gracefully
        buf.show_message("WAY_TOO_LONG_MESSAGE");
        let f = buf.snapshot();
        assert_eq!(f.width, 4);
        assert_eq!(f.height, 2);

        // Multiline show_message
        buf.show_message("Line 1\nLine 2\nLine 3\nLine 4");
        let f_multi = buf.snapshot();
        assert_eq!(f_multi.height, 2);
    }

    #[test]
    fn screen_buffer_additional_edge_cases() {
        let mut buf = ScreenBuffer::new(3, 2);

        // 1. Partial ESC at end of buffer
        buf.feed(b"AB\x1b");
        assert_eq!(buf.pending, vec![0x1b]);
        buf.feed(b"[;HC");
        assert_eq!(frame_text(&buf), "CB \n   ");

        // 2. Partial OSC at end of buffer (missing terminator)
        let mut buf2 = ScreenBuffer::new(3, 2);
        buf2.feed(b"\x1b]2;incomplete");
        assert_eq!(buf2.pending, b"\x1b]2;incomplete".to_vec());
        buf2.feed(b"\x07X");
        assert_eq!(frame_text(&buf2), "X  \n   ");

        // 3. Partial ST in OSC (ending in ESC)
        let mut buf3 = ScreenBuffer::new(3, 2);
        buf3.feed(b"\x1b]2;title\x1b");
        assert_eq!(buf3.pending, b"\x1b]2;title\x1b".to_vec());
        buf3.feed(b"\\Y");
        assert_eq!(frame_text(&buf3), "Y  \n   ");

        // 4. Two-byte escape sequence (like ESC M or ESC \)
        let mut buf4 = ScreenBuffer::new(3, 2);
        buf4.feed(b"\x1bMhello");
        assert_eq!(frame_text(&buf4), "hel\n   ");

        // 5. Incomplete CSI terminator at end of buffer
        let mut buf5 = ScreenBuffer::new(3, 2);
        buf5.feed(b"\x1b[38;2;10;20;");
        assert_eq!(buf5.pending, b"\x1b[38;2;10;20;".to_vec());
        buf5.feed(b"30mZ");
        let snap = buf5.snapshot();
        assert_eq!(snap.cells[0].color, Some(RgbColor::new(10, 20, 30)));
        assert_eq!(snap.cells[0].ch, 'Z');

        // 6. Invalid UTF-8 sequence error branch
        // 0xE0 followed by invalid continuation byte 0x00
        let mut buf6 = ScreenBuffer::new(3, 2);
        buf6.feed(b"\xe0\x00\x00A");
        assert_eq!(frame_text(&buf6), "A  \n   ");

        // 7. Place when cursor_row >= height
        let mut small_buf = ScreenBuffer::new(2, 1);
        small_buf.feed(b"\n\n\nXYZ"); // cursor_row moves to 3 >= height 1

        // 8. Place when cursor_col >= width on same line (no auto-wrap)
        let mut row_buf = ScreenBuffer::new(2, 2);
        row_buf.feed(b"ABCDEF");
        assert_eq!(frame_text(&row_buf), "AB\n  ");

        // 9. Erase line (CSI K) with default mode (no param) and mode 0
        let mut erase_buf = ScreenBuffer::new(4, 2);
        erase_buf.feed(b"ABCD\nEFGH");
        erase_buf.feed(b"\x1b[1;2H\x1b[K"); // default erase to end of line
        assert_eq!(frame_text(&erase_buf), "A   \nEFGH");

        // 10. Erase line when cursor_row >= height
        erase_buf.feed(b"\x1b[10;1H\x1b[K");

        // 11. Erase line when cursor_col >= width
        let mut erase_buf2 = ScreenBuffer::new(2, 2);
        erase_buf2.feed(b"AB\nCD");
        erase_buf2.feed(b"\x1b[1;10H\x1b[K");

        // 12. CSI J default mode / any mode
        let mut clear_buf = ScreenBuffer::new(2, 2);
        clear_buf.feed(b"AB\nCD");
        clear_buf.feed(b"\x1b[J");
        assert_eq!(frame_text(&clear_buf), "  \n  ");

        // 13. Cursor position CSI H with partial numbers (e.g. \x1b[;5H, \x1b[10;H, \x1b[H)
        let mut cur_buf = ScreenBuffer::new(5, 5);
        cur_buf.feed(b"\x1b[;3HA"); // row 1, col 3
        assert_eq!(
            cur_buf.snapshot().cells[cur_buf.snapshot().idx(2, 0)].ch,
            'A'
        );
        cur_buf.feed(b"\x1b[3;HB"); // row 3, col 1
        assert_eq!(
            cur_buf.snapshot().cells[cur_buf.snapshot().idx(0, 2)].ch,
            'B'
        );
        cur_buf.feed(b"\x1b[HC"); // row 1, col 1
        assert_eq!(
            cur_buf.snapshot().cells[cur_buf.snapshot().idx(0, 0)].ch,
            'C'
        );

        // 14. Unknown CSI command
        cur_buf.feed(b"\x1b[?25h"); // cursor visible sequence, unhandled CSI
    }

    #[test]
    fn parse_int_field_and_utf8_length_unit_tests() {
        // parse_int_field with non-digit
        assert_eq!(parse_int_field(b"12a3"), None);
        assert_eq!(parse_int_field(b""), None);
        assert_eq!(parse_int_field(b"123"), Some(123));

        // utf8_length for 2, 3, 4 bytes and 1 byte fallback
        assert_eq!(utf8_length(0b1100_0000), 2);
        assert_eq!(utf8_length(0b1110_0000), 3);
        assert_eq!(utf8_length(0b1111_0000), 4);
        assert_eq!(utf8_length(0b1111_1000), 1);
        assert_eq!(utf8_length(0x41), 1);
    }
}
