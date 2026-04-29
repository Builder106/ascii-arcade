import XCTest
@testable import DonutCore

final class HelixFrameGeneratorTests: XCTestCase {
    func testFrameProducesCorrectDimensions() {
        let width = 80, height = 22
        let gen = HelixFrameGenerator(width: width, height: height)
        let frame = gen.frame(atTime: 0.0)
        let lines = frame.split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.count, height)
        for line in lines { XCTAssertEqual(line.count, width) }
    }

    func testFrameContainsOnlyExpectedCharacters() {
        let gen = HelixFrameGenerator(width: 80, height: 22)
        let frame = gen.frame(atTime: 0.0)
        let charset = Set(".,-~:;=!*#$@ ")
        XCTAssertTrue(frame.allSatisfy { $0 == "\n" || charset.contains($0) })
    }

    func testDifferentTimesProduceDifferentFrames() {
        let gen = HelixFrameGenerator(width: 80, height: 22)
        XCTAssertNotEqual(gen.frame(atTime: 0.0), gen.frame(atTime: 0.1))
    }

    func testVerticalCoverageOverRotation() {
        let width = 200, height = 35
        let gen = HelixFrameGenerator(width: width, height: height)
        var maxSpan = 0
        var t = 0.0
        while t < 6.5 {
            let frame = gen.frame(atTime: t)
            let lines = frame.split(separator: "\n", omittingEmptySubsequences: false)
            let inkedRows = lines.enumerated().compactMap { idx, line -> Int? in
                line.contains(where: { $0 != " " }) ? idx : nil
            }
            if let minRow = inkedRows.min(), let maxRow = inkedRows.max() {
                maxSpan = max(maxSpan, maxRow - minRow + 1)
            }
            t += 0.35
        }
        XCTAssertGreaterThanOrEqual(maxSpan, (height * 2) / 3)
    }
}

final class DonutCoreTests: XCTestCase {
    func testFrameGeneratorProducesCorrectDimensions() throws {
        let width = 80
        let height = 22
        let generator = DonutFrameGenerator(width: width, height: height)
        let frame = generator.frame(atTime: 0.0)
        let lines = frame.split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.count, height, "Frame should have correct number of lines")
        for line in lines {
            XCTAssertEqual(line.count, width, "Each line should have correct width")
        }
    }

    func testFrameContainsBrightnessCharacters() throws {
        let generator = DonutFrameGenerator(width: 80, height: 22)
        let frame = generator.frame(atTime: 0.0)
        let charset = Set(".,-~:;=!*#$@ ")
        XCTAssertTrue(frame.allSatisfy { $0 == "\n" || charset.contains($0) }, "Only expected ASCII characters should appear")
        XCTAssertTrue(frame.contains("@") || frame.contains("$") || frame.contains("#"), "Frame should contain some bright characters at t=0")
    }

    func testDifferentTimesProduceDifferentFrames() throws {
        let generator = DonutFrameGenerator(width: 80, height: 22)
        let f0 = generator.frame(atTime: 0.0)
        let f1 = generator.frame(atTime: 0.1)
        XCTAssertNotEqual(f0, f1, "Frames at different times should differ")
    }

    func testGridDimensionsMatchesLayoutFormula() throws {
        let paddedWidth = 800.0
        let paddedHeight = 400.0
        let charWidth = 8.0
        let lineHeight = 12.0
        let expectedW = max(10, Int(paddedWidth / charWidth))
        let expectedH = max(10, Int(paddedHeight / lineHeight))
        let d = DonutFrameGenerator.gridDimensions(
            paddedWidth: paddedWidth,
            paddedHeight: paddedHeight,
            charWidth: charWidth,
            lineHeight: lineHeight
        )
        XCTAssertEqual(d.width, expectedW)
        XCTAssertEqual(d.height, expectedH)
    }

    func testGridDimensionsEnforcesMinimum() throws {
        let d = DonutFrameGenerator.gridDimensions(
            paddedWidth: 30.0,
            paddedHeight: 30.0,
            charWidth: 10.0,
            lineHeight: 10.0
        )
        XCTAssertEqual(d.width, 10)
        XCTAssertEqual(d.height, 10)
    }

    func testGridDimensionsClampsCharWidthButNotLineHeightToOne() throws {
        let d = DonutFrameGenerator.gridDimensions(
            paddedWidth: 100.0,
            paddedHeight: 50.0,
            charWidth: 0.5,
            lineHeight: 0.5
        )
        let expectedW = max(10, Int(100.0 / max(1.0, 0.5)))
        let expectedH = max(10, Int(50.0 / 0.5))
        XCTAssertEqual(d.width, expectedW)
        XCTAssertEqual(d.height, expectedH)
    }

    func testWideShortGridDonutSpansMostOfTheHeightOverRotation() throws {
        let width = 200
        let height = 35
        let generator = DonutFrameGenerator(width: width, height: height)
        var maxSpan = 0
        var t = 0.0
        while t < 6.5 {
            let frame = generator.frame(atTime: t)
            let lines = frame.split(separator: "\n", omittingEmptySubsequences: false)
            let rowsWithInk = lines.enumerated().compactMap { index, line -> Int? in
                line.contains(where: { $0 != " " }) ? index : nil
            }
            if let minRow = rowsWithInk.min(), let maxRow = rowsWithInk.max() {
                maxSpan = max(maxSpan, maxRow - minRow + 1)
            }
            t += 0.35
        }
        XCTAssertGreaterThanOrEqual(maxSpan, (height * 2) / 3)
    }
}
