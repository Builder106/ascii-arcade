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

    func testResizeClampsDimensionsAndDropsPendingBytes() {
        let buf = DoomScreenBuffer(width: 4, height: 2)
        buf.feed([0x1b])
        buf.resize(width: 0, height: -1)
        XCTAssertEqual(buf.coloredSnapshot().width, 1)
        XCTAssertEqual(buf.coloredSnapshot().height, 1)
        XCTAssertEqual(buf.snapshot(), " ")

        buf.feed(Array("[;HX".utf8))
        XCTAssertEqual(buf.snapshot(), "[")

        buf.resize(width: 4, height: 2)
        XCTAssertEqual(buf.snapshot(), "    \n    ")
    }

    func testShowMessageCentersAndClipsText() {
        let buf = DoomScreenBuffer(width: 10, height: 3)
        buf.showMessage("hi")
        XCTAssertEqual(buf.snapshot(), "          \n    hi    \n          ")

        buf.showMessage("01234567890")
        XCTAssertEqual(buf.snapshot(), "          \n0123456789\n          ")
    }

    func testEscapeSequencesCanBeSplitAndIgnored() {
        let buf = DoomScreenBuffer(width: 8, height: 2)
        buf.feed([0x1b])
        buf.feed(Array("[2;3HAB".utf8))
        XCTAssertEqual(buf.snapshot(), "        \n  AB    ")

        buf.feed([0x1b, 0x5b])
        buf.feed(Array("1;1Hc".utf8))
        XCTAssertEqual(buf.snapshot(), "c       \n  AB    ")

        buf.feed(Array("\u{1b}c\u{1b}]0;title\u{07}X".utf8))
        XCTAssertEqual(buf.snapshot(), "cX      \n  AB    ")

        buf.feed(Array("\u{1b}]0;title\u{1b}\\Y".utf8))
        XCTAssertEqual(buf.snapshot(), "cXY     \n  AB    ")
    }

    func testPartialOSCAndControlBytesAreHandled() {
        let buf = DoomScreenBuffer(width: 5, height: 1)
        buf.feed(Array("A\u{1b}]title".utf8) + [0x1b])
        buf.feed([0x5c] + [0x0d, 0x09, 0x01] + Array("B".utf8))
        XCTAssertEqual(buf.snapshot(), "B    ")

        buf.feed([0x1b, 0x5d] + Array("unterminated".utf8))
        buf.feed([0x07])
        XCTAssertEqual(buf.snapshot(), "B    ")
    }

    func testCursorModesEraseLinesAndClampPositions() {
        let buf = DoomScreenBuffer(width: 6, height: 2)
        buf.feed(Array("abcdef\nuvwxyz".utf8))
        buf.feed(Array("\u{1b}[1;3H\u{1b}[1K".utf8))
        XCTAssertEqual(buf.snapshot(), "   def\nuvwxyz")

        buf.feed(Array("\u{1b}[2;1H\u{1b}[2K".utf8))
        XCTAssertEqual(buf.snapshot(), "   def\n      ")

        buf.feed(Array("\u{1b}[1;2H\u{1b}[0K".utf8))
        XCTAssertEqual(buf.snapshot(), "      \n      ")

        buf.feed(Array("\u{1b}[1;99H\u{1b}[0K".utf8))
        XCTAssertEqual(buf.snapshot(), "      \n      ")

        buf.feed(Array("\u{1b}[0;0Hq\u{1b}[999;999fR".utf8))
        XCTAssertEqual(buf.snapshot(), "q     \n     R")

        buf.feed(Array("\u{1b}[?c".utf8))
        buf.feed(Array("\u{1b}[?K".utf8))
    }

    func testSGRVariantsClampAndResetColors() {
        let buf = DoomScreenBuffer(width: 6, height: 1)
        buf.feed(Array("\u{1b}[38;2;-5;300;7mA".utf8))
        XCTAssertEqual(buf.coloredSnapshot().colors[0], RGBColor(r: 0, g: 255, b: 7))

        buf.feed(Array("\u{1b}[39mB\u{1b}[38;5;4mC".utf8))
        let first = buf.coloredSnapshot()
        XCTAssertNil(first.colors[1])
        XCTAssertNil(first.colors[2])

        buf.feed(Array("\u{1b}[38;2;1;2;3;39mD".utf8))
        let second = buf.coloredSnapshot()
        XCTAssertNil(second.colors[3])

        buf.feed(Array("\u{1b}[xmE\u{1b}[1;2;3mF\u{1b}[mG".utf8))
        XCTAssertNil(buf.coloredSnapshot().colors[4])
    }

    func testUTF8VariantsAndOutOfBoundsWrites() {
        let buf = DoomScreenBuffer(width: 3, height: 1)
        buf.feed(Array("é😀".utf8))
        buf.feed([0xc2, 0x20, 0xff, 0xe2])
        buf.feed([0x96, 0x88])
        buf.feed(Array("XYZ".utf8))
        XCTAssertEqual(buf.snapshot(), "é😀█")

        let one = DoomScreenBuffer(width: 1, height: 1)
        one.feed(Array("x\n\ny".utf8))
        XCTAssertEqual(one.snapshot(), "x")
        one.feed(Array("\u{1b}[2K".utf8))
        XCTAssertEqual(one.snapshot(), "x")
    }
}
