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

private final class NilColoredFrameScene: SteppedScene {
    override func coloredFrame(atTime t: Double) -> ColoredFrame? {
        nil
    }
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

        let color = RGBColor(r: 10, g: 20, b: 30)
        XCTAssertEqual(color.scaled(-1.0), .black)
        XCTAssertEqual(color.scaled(2.0), color)
        XCTAssertEqual(color.mixed(with: white, t: -1.0), color)
        XCTAssertEqual(color.mixed(with: white, t: 2.0), white)
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

final class SceneDefaultsTests: XCTestCase {
    private final class DefaultScene: AsciiScene {
        let displayName = "Default"

        func setGrid(width: Int, height: Int) {}

        func frame(atTime t: Double) -> String { "" }
    }

    func testGeneratorSceneHasNoColoredFrame() {
        let s = GeneratorScene(displayName: "Donut") { w, h in DonutFrameGenerator(width: w, height: h) }
        s.setGrid(width: 40, height: 12)
        XCTAssertNil(s.coloredFrame(atTime: 0.0), "math scenes stay monochrome")
    }

    func testSceneDefaultsAreInertAndMonochrome() {
        let scene = DefaultScene()
        XCTAssertFalse(scene.isInteractive)
        XCTAssertNil(scene.fixedGrid)
        XCTAssertNil(scene.coloredFrame(atTime: 0.0))
        XCTAssertTrue(scene.settings.isEmpty)
        scene.applyBaseColor(.white)
        scene.applySetting(id: "unused", value: 1)
        scene.sendKey([0x1B])
        scene.start()
        scene.stop()
    }

    func testZeroSeedUsesTheDocumentedNonzeroState() {
        var zeroSeed = SeededGenerator(seed: 0)
        var explicitDefault = SeededGenerator(seed: 0x9E37_79B9_7F4A_7C15)
        XCTAssertEqual(zeroSeed.next(), explicitDefault.next())
    }

    func testSceneOptionAndSceneSettingClamping() {
        let opt1 = SceneOption(label: "Slow", value: 0.5)
        let opt2 = SceneOption(label: "Fast", value: 2.0)
        XCTAssertEqual(opt1.label, "Slow")
        XCTAssertEqual(opt1.value, 0.5)
        XCTAssertEqual(opt1, SceneOption(label: "Slow", value: 0.5))

        let setting = SceneSetting(id: "speed", label: "Speed", options: [opt1, opt2], defaultIndex: 99)
        XCTAssertEqual(setting.defaultIndex, 1)
        let settingLow = SceneSetting(id: "speed", label: "Speed", options: [opt1, opt2], defaultIndex: -5)
        XCTAssertEqual(settingLow.defaultIndex, 0)
    }

    func testSteppedSceneBaseClassBehaviors() {
        let s = SteppedScene(displayName: "BaseStepped", initialWidth: 10, initialHeight: 5)
        XCTAssertEqual(s.displayName, "BaseStepped")
        XCTAssertEqual(s.width, 10)
        XCTAssertEqual(s.height, 5)
        XCTAssertEqual(s.settingValue("unknown", default: 42.0), 42.0)

        // Invalid or identical grid dimensions are ignored
        s.setGrid(width: 0, height: 5)
        XCTAssertEqual(s.width, 10)
        s.setGrid(width: 10, height: 5)
        XCTAssertEqual(s.width, 10)

        s.setGrid(width: 12, height: 6)
        XCTAssertEqual(s.width, 12)
        XCTAssertEqual(s.height, 6)

        let color = RGBColor(r: 12, g: 34, b: 56)
        s.applyBaseColor(color)
        XCTAssertEqual(s.baseColor, color)
        s.applySetting(id: "speed", value: 2.0)
        XCTAssertEqual(s.settingValue("speed", default: 0.0), 2.0)

        // Test frame and coloredFrame
        let f1 = s.frame(atTime: 1.0)
        XCTAssertFalse(f1.isEmpty)

        // Advance clock backwards (dt < 0) and large dt (> 0.25)
        _ = s.coloredFrame(atTime: 0.5)
        _ = s.coloredFrame(atTime: 5.0)

        s.start()
        s.stop()

        let nilScene = NilColoredFrameScene(displayName: "NilFrame", initialWidth: 2, initialHeight: 2)
        XCTAssertEqual(nilScene.frame(atTime: 0.0), "  \n  ")
    }
}
