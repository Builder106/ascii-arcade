import XCTest
import AsciiArcadeCore

final class DoomScreenBufferTests: XCTestCase {
    /// A blank buffer is a full grid of spaces with the right dimensions.
    func testSnapshotDimensions() {
        let buf = DoomScreenBuffer(width: 10, height: 3)
        let lines = buf.snapshot().split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.count, 3)
        for line in lines { XCTAssertEqual(line.count, 10) }
    }

    /// Cursor-home + plain text lands at the top-left and survives a snapshot.
    func testHomeAndWrite() {
        let buf = DoomScreenBuffer(width: 8, height: 2)
        buf.feed(Array("\u{1b}[;HAB\nCD".utf8))
        let lines = buf.snapshot().split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(String(lines[0]), "AB      ")
        XCTAssertEqual(String(lines[1]), "CD      ")
    }

    /// SGR colour/style codes are stripped; only the glyphs remain.
    func testStripsSGRColorCodes() {
        let buf = DoomScreenBuffer(width: 6, height: 1)
        // \033[1m bold, \033[38;2;255;0;0m red, glyph 'X', \033[0m reset
        buf.feed(Array("\u{1b}[;H\u{1b}[1m\u{1b}[38;2;255;0;0mX\u{1b}[0m".utf8))
        XCTAssertEqual(buf.snapshot(), "X     ")
    }

    /// A `\033[2J` clear blanks the grid.
    func testClearScreen() {
        let buf = DoomScreenBuffer(width: 4, height: 1)
        buf.feed(Array("\u{1b}[;HZZZZ".utf8))
        XCTAssertEqual(buf.snapshot(), "ZZZZ")
        buf.feed(Array("\u{1b}[2J".utf8))
        XCTAssertEqual(buf.snapshot(), "    ")
    }

    /// A full redraw (home then content) replaces the previous frame's cells.
    func testFullRedrawOverwrites() {
        let buf = DoomScreenBuffer(width: 5, height: 1)
        buf.feed(Array("\u{1b}[;Hhello".utf8))
        XCTAssertEqual(buf.snapshot(), "hello")
        buf.feed(Array("\u{1b}[;Hworld".utf8))
        XCTAssertEqual(buf.snapshot(), "world")
    }

    /// A multibyte UTF-8 block glyph split across two feeds is reassembled.
    func testMultibyteGlyphAcrossChunkBoundary() {
        let buf = DoomScreenBuffer(width: 3, height: 1)
        let full = Array("\u{1b}[;H█".utf8) // █ = E2 96 88
        let split = full.count - 1
        buf.feed(Array(full[..<split]))   // ESC[;H + first byte(s) of glyph
        buf.feed(Array(full[split...]))   // trailing continuation byte
        XCTAssertEqual(buf.snapshot(), "█  ")
    }

    /// `38;2;R;G;B` truecolor is retained on the cell it precedes; `0` resets it.
    func testColoredSnapshotRetainsTruecolor() {
        let buf = DoomScreenBuffer(width: 3, height: 1)
        buf.feed(Array("\u{1b}[;H\u{1b}[38;2;255;0;0mX\u{1b}[0mY".utf8))
        let f = buf.coloredSnapshot()
        XCTAssertEqual(f.chars[0], "X")
        XCTAssertEqual(f.colors[0], RGBColor(r: 255, g: 0, b: 0))
        XCTAssertEqual(f.chars[1], "Y")
        XCTAssertNil(f.colors[1], "after reset the colour clears")
        // The monochrome path is unchanged.
        XCTAssertEqual(buf.snapshot(), "XY ")
    }

    /// Clearing the screen also clears the colour grid.
    func testClearResetsColors() {
        let buf = DoomScreenBuffer(width: 2, height: 1)
        buf.feed(Array("\u{1b}[;H\u{1b}[38;2;10;20;30mAB".utf8))
        XCTAssertEqual(buf.coloredSnapshot().colors[0], RGBColor(r: 10, g: 20, b: 30))
        buf.clear()
        XCTAssertTrue(buf.coloredSnapshot().colors.allSatisfy { $0 == nil })
    }
}
