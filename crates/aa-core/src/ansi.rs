use crate::{Frame, RgbColor, Theme};
use std::fmt::Write as FmtWrite;

/// Encode `frame` as a stream of ANSI truecolor escape sequences suitable for
/// an xterm-compatible terminal. Each call emits a full-screen redraw:
///
/// * `\x1b[2J\x1b[H` — clear the screen and home the cursor.
/// * Per non-space cell: `\x1b[38;2;R;G;Bm<ch>` (foreground only; the caller
///   is responsible for setting the terminal's background colour via its theme
///   options so that space cells show the correct backdrop without per-cell
///   background escape codes).
/// * Adjacent cells with the same colour share one escape prefix (run encoding).
/// * `\x1b[0m` at the end resets all attributes.
///
/// The output is valid UTF-8 (cells are plain ASCII glyphs or spaces).
pub fn frame_to_ansi(frame: &Frame, theme: &Theme) -> String {
    let cap = frame.width * frame.height * 6 + frame.height * 2 + 16;
    let mut buf = String::with_capacity(cap);

    buf.push_str("\x1b[2J\x1b[H");

    let mut current: Option<RgbColor> = None;

    for row in 0..frame.height {
        for col in 0..frame.width {
            let cell = frame.cells[frame.idx(col, row)];
            if cell.ch == ' ' {
                // Space cells reveal the terminal background — skip the colour
                // escape and reset tracking so the next non-space gets a fresh
                // escape.
                buf.push(' ');
                current = None;
            } else {
                let color = cell.color.unwrap_or(theme.text);
                if current != Some(color) {
                    let _ = write!(buf, "\x1b[38;2;{};{};{}m", color.r, color.g, color.b);
                    current = Some(color);
                }
                buf.push(cell.ch);
            }
        }
        if row + 1 < frame.height {
            buf.push_str("\r\n");
        }
    }

    buf.push_str("\x1b[0m");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, Frame, Theme};

    #[test]
    fn blank_frame_contains_clear_and_reset() {
        let f = Frame::blank(4, 2);
        let out = frame_to_ansi(&f, &Theme::HACKER);
        assert!(out.starts_with("\x1b[2J\x1b[H"));
        assert!(out.ends_with("\x1b[0m"));
    }

    #[test]
    fn non_blank_cell_emits_truecolor_escape_and_char() {
        let mut f = Frame::blank(1, 1);
        f.set(0, 0, Cell::new('X', Some(RgbColor::new(10, 20, 30))));
        let out = frame_to_ansi(&f, &Theme::HACKER);
        assert!(out.contains("\x1b[38;2;10;20;30mX"));
    }

    #[test]
    fn consecutive_same_color_cells_share_one_escape() {
        let mut f = Frame::blank(3, 1);
        let color = RgbColor::new(0, 255, 0);
        for x in 0..3 {
            f.set(x, 0, Cell::new('A', Some(color)));
        }
        let out = frame_to_ansi(&f, &Theme::HACKER);
        // Only one colour-escape prefix for three cells.
        assert_eq!(out.matches("\x1b[38;2;").count(), 1);
    }

    #[test]
    fn space_cell_emits_no_escape() {
        let f = Frame::blank(2, 1); // both blank (space)
        let out = frame_to_ansi(&f, &Theme::HACKER);
        assert!(!out.contains("\x1b[38;2;"));
    }
}
