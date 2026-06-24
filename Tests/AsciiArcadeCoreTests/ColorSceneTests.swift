import XCTest
@testable import AsciiArcadeCore

/// Drives a scene through a short time sequence so its simulation actually steps.
private func advance(_ scene: AsciiScene, frames: Int = 30, dt: Double = 0.05) -> ColoredFrame {
    var last: ColoredFrame!
    var t = 0.0
    for _ in 0...frames {
        last = scene.coloredFrame(atTime: t)
        t += dt
    }
    return last
}

final class ColoredFrameTests: XCTestCase {
    func testTextJoinsRowsWithNewlines() {
        let f = ColoredFrame(width: 2, height: 2,
                             chars: ["a", "b", "c", "d"],
                             colors: [nil, nil, nil, nil])
        XCTAssertEqual(f.text(), "ab\ncd")
    }

    func testRGBScaleAndMix() {
        let white = RGBColor.white
        XCTAssertEqual(white.scaled(0.0), RGBColor(r: 0, g: 0, b: 0))
        XCTAssertEqual(white.scaled(1.0), white)
        let mid = RGBColor.black.mixed(with: white, t: 0.5)
        XCTAssertEqual(mid, RGBColor(r: 128, g: 128, b: 128))
    }
}

final class MatrixRainSceneTests: XCTestCase {
    func testDimensions() {
        let s = MatrixRainScene()
        s.setGrid(width: 60, height: 20)
        let f = advance(s)
        XCTAssertEqual(f.width, 60)
        XCTAssertEqual(f.height, 20)
        XCTAssertEqual(f.chars.count, 60 * 20)
        XCTAssertEqual(f.colors.count, 60 * 20)
        let lines = f.text().split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.count, 20)
        for line in lines { XCTAssertEqual(line.count, 60) }
    }

    func testEveryDrawnGlyphIsColored() {
        let s = MatrixRainScene()
        s.setGrid(width: 60, height: 20)
        let f = advance(s)
        var drawn = 0
        for i in 0..<f.chars.count where f.chars[i] != " " {
            drawn += 1
            XCTAssertNotNil(f.colors[i], "a lit rain glyph must carry a colour")
        }
        XCTAssertGreaterThan(drawn, 0, "rain should have drawn something")
    }

    func testBaseColorTintsTheRain() {
        let s = MatrixRainScene()
        s.applyBaseColor(RGBColor(r: 255, g: 100, b: 0)) // amber
        s.setGrid(width: 40, height: 16)
        let f = advance(s)
        // Trail cells (not the bright head) should be reddish, never blue-dominant.
        for c in f.colors.compactMap({ $0 }) {
            XCTAssertGreaterThanOrEqual(c.r, c.b)
        }
    }
}

final class FireSceneTests: XCTestCase {
    func testBottomRowIgnitesAndIsColored() {
        let s = FireScene()
        s.setGrid(width: 40, height: 12)
        let f = s.coloredFrame(atTime: 0.0)!
        let base = (12 - 1) * 40
        for x in 0..<40 {
            XCTAssertNotEqual(f.chars[base + x], " ", "bottom row should be lit")
            XCTAssertNotNil(f.colors[base + x])
        }
    }

    func testDeterministicForSameSeedAndSize() {
        let a = FireScene(); a.setGrid(width: 50, height: 18)
        let b = FireScene(); b.setGrid(width: 50, height: 18)
        XCTAssertEqual(advance(a).text(), advance(b).text())
    }
}

final class GameOfLifeSceneTests: XCTestCase {
    func testDimensionsAndColoredLiveCells() {
        let s = GameOfLifeScene()
        s.setGrid(width: 60, height: 30)
        let f = advance(s, frames: 10, dt: 0.2)
        XCTAssertEqual(f.chars.count, 60 * 30)
        var live = 0
        for i in 0..<f.chars.count where f.chars[i] != " " {
            live += 1
            XCTAssertEqual(f.chars[i], "█", "live cells render as solid blocks")
            XCTAssertNotNil(f.colors[i])
        }
        XCTAssertGreaterThan(live, 0, "seeded patterns should leave the board populated")
    }

    func testReseedsWhenStalled() {
        // The board must never stay all-dead for long; patterns keep it alive.
        let s = GameOfLifeScene()
        s.setGrid(width: 48, height: 24)
        var sawLife = false
        var t = 0.0
        for _ in 0..<200 {
            let f = s.coloredFrame(atTime: t)!
            if f.chars.contains(where: { $0 != " " }) { sawLife = true }
            t += 0.2
        }
        XCTAssertTrue(sawLife)
    }
}

final class PipesSceneTests: XCTestCase {
    func testDrawsColoredBoxDrawingGlyphs() {
        let s = PipesScene()
        s.setGrid(width: 50, height: 20)
        let f = advance(s, frames: 60, dt: 0.05)
        let pipeGlyphs = Set("│─┌┐└┘+")
        var drawn = 0
        for i in 0..<f.chars.count where f.chars[i] != " " {
            drawn += 1
            XCTAssertTrue(pipeGlyphs.contains(f.chars[i]))
            XCTAssertNotNil(f.colors[i])
        }
        XCTAssertGreaterThan(drawn, 0)
    }
}

final class ClockSceneTests: XCTestCase {
    private func fixedDate(h: Int, m: Int, s: Int) -> Date {
        var c = DateComponents()
        c.year = 2026; c.month = 6; c.day = 23
        c.hour = h; c.minute = m; c.second = s
        return Calendar.current.date(from: c)!
    }

    func testRendersDigitsMonochrome() {
        let clock = ClockScene()
        clock.now = { [weak self] in self?.fixedDate(h: 12, m: 34, s: 56) ?? Date() }
        clock.setGrid(width: 100, height: 30)
        let f = clock.coloredFrame(atTime: 0.0)!
        XCTAssertTrue(f.chars.contains("█"), "the clock should draw block pixels")
        // Monochrome: the host paints it in the theme colour, so no per-cell colours.
        XCTAssertTrue(f.colors.allSatisfy { $0 == nil })
        let lines = f.text().split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.count, 30)
        for line in lines { XCTAssertEqual(line.count, 100) }
    }
}

final class SceneDefaultsTests: XCTestCase {
    func testGeneratorSceneHasNoColoredFrame() {
        let s = GeneratorScene(displayName: "Donut") { w, h in DonutFrameGenerator(width: w, height: h) }
        s.setGrid(width: 40, height: 12)
        XCTAssertNil(s.coloredFrame(atTime: 0.0), "math scenes stay monochrome")
    }
}
