import XCTest
@testable import Hotword

final class HotwordTests: XCTestCase {
    func testDetectsDoomSequence() throws {
        var detector = HotwordDetector(hotword: "doom", isCaseSensitive: false, windowMs: 3000)
        let base = 0.0
        XCTAssertFalse(detector.push("d", timestampMs: base + 10))
        XCTAssertFalse(detector.push("o", timestampMs: base + 20))
        XCTAssertFalse(detector.push("o", timestampMs: base + 30))
        XCTAssertTrue(detector.push("m", timestampMs: base + 40))
        // Further characters should not re-trigger without new sequence
        XCTAssertFalse(detector.push("x", timestampMs: base + 50))
    }

    func testCaseInsensitive() throws {
        var detector = HotwordDetector(hotword: "doom", isCaseSensitive: false, windowMs: 3000)
        let base = 100.0
        XCTAssertFalse(detector.push("D", timestampMs: base + 10))
        XCTAssertFalse(detector.push("O", timestampMs: base + 20))
        XCTAssertFalse(detector.push("o", timestampMs: base + 30))
        XCTAssertTrue(detector.push("M", timestampMs: base + 40))
    }

    func testTimeoutResetsProgress() throws {
        var detector = HotwordDetector(hotword: "doom", isCaseSensitive: false, windowMs: 100)
        let base = 200.0
        XCTAssertFalse(detector.push("d", timestampMs: base + 0))
        XCTAssertFalse(detector.push("o", timestampMs: base + 50))
        // timeout
        XCTAssertFalse(detector.push("o", timestampMs: base + 500))
        // After timeout, sequence should restart from first char if matching
        XCTAssertFalse(detector.push("o", timestampMs: base + 510))
        XCTAssertFalse(detector.push("m", timestampMs: base + 520))
        // Start correct sequence again
        XCTAssertFalse(detector.push("d", timestampMs: base + 530))
        XCTAssertFalse(detector.push("o", timestampMs: base + 540))
        XCTAssertFalse(detector.push("o", timestampMs: base + 550))
        XCTAssertTrue(detector.push("m", timestampMs: base + 560))
    }

    func testOverlappingSequences() throws {
        var detector = HotwordDetector(hotword: "aba", isCaseSensitive: true, windowMs: 1000)
        let base = 300.0
        XCTAssertFalse(detector.push("a", timestampMs: base + 1))
        XCTAssertFalse(detector.push("b", timestampMs: base + 2))
        XCTAssertTrue(detector.push("a", timestampMs: base + 3))
        // overlapping: last 'a' can be start of next sequence
        XCTAssertFalse(detector.push("b", timestampMs: base + 4))
        XCTAssertTrue(detector.push("a", timestampMs: base + 5))
    }

    func testIgnoresNonMatchingAndResynchronizes() throws {
        var detector = HotwordDetector(hotword: "doom", isCaseSensitive: false, windowMs: 1000)
        let base = 400.0
        XCTAssertFalse(detector.push("x", timestampMs: base + 1))
        XCTAssertFalse(detector.push("d", timestampMs: base + 2))
        XCTAssertFalse(detector.push("x", timestampMs: base + 3))
        XCTAssertFalse(detector.push("o", timestampMs: base + 4))
        XCTAssertFalse(detector.push("o", timestampMs: base + 5))
        XCTAssertTrue(detector.push("m", timestampMs: base + 6))
    }
}
