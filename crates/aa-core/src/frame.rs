//! One rendered frame: a `width × height` grid of cells.
//!
//! This unifies the Swift side's split between `frame(atTime:) -> String`
//! (monochrome) and `coloredFrame(atTime:) -> ColoredFrame?` (per-cell colour).
//! A [`Cell`] carries an optional colour: `None` means "use the theme's text
//! colour", exactly like `ColoredFrame.colors` holding `nil`. A monochrome scene
//! simply leaves every cell's colour `None`.

use crate::color::RgbColor;

/// A single character cell. `color == None` ⇒ paint in the active theme colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub color: Option<RgbColor>,
}

impl Cell {
    pub const BLANK: Cell = Cell { ch: ' ', color: None };

    pub const fn new(ch: char, color: Option<RgbColor>) -> Self {
        Cell { ch, color }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::BLANK
    }
}

/// A row-major grid of [`Cell`]s. `cells.len() == width * height`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

impl Frame {
    /// A frame of all-blank cells.
    pub fn blank(width: usize, height: usize) -> Self {
        Frame {
            width,
            height,
            cells: vec![Cell::BLANK; width * height],
        }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Write a cell if `(x, y)` is in bounds; out-of-bounds writes are ignored
    /// (scenes project floating-point coordinates and can land just outside).
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            let i = self.idx(x, y);
            self.cells[i] = cell;
        }
    }

    #[inline]
    pub fn set_char(&mut self, x: usize, y: usize, ch: char) {
        self.set(x, y, Cell::new(ch, None));
    }

    /// The glyphs as `height` newline-joined rows — the monochrome view, used by
    /// the headless renderer, the C-ABI text export, and parity tests.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity((self.width + 1) * self.height);
        for row in 0..self.height {
            let start = row * self.width;
            out.extend(self.cells[start..start + self.width].iter().map(|c| c.ch));
            if row + 1 < self.height {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_joins_rows() {
        let mut f = Frame::blank(2, 2);
        f.set_char(0, 0, 'a');
        f.set_char(1, 0, 'b');
        f.set_char(0, 1, 'c');
        f.set_char(1, 1, 'd');
        assert_eq!(f.text(), "ab\ncd");
    }

    #[test]
    fn out_of_bounds_set_is_ignored() {
        let mut f = Frame::blank(2, 2);
        f.set_char(5, 5, 'x');
        assert_eq!(f.text(), "  \n  ");
    }
}
