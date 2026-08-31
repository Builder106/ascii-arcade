import XCTest
@testable import AsciiArcadeCore

final class SceneTransitionTests: XCTestCase {
    func testThresholdsAreDeterministicForTheSameSeed() {
        let first = SceneTransition(cellCount: 64, seed: 42)
        let second = SceneTransition(cellCount: 64, seed: 42)

        for index in 0..<64 {
            XCTAssertEqual(
                first.usesDestination(at: index, progress: 0.37),
                second.usesDestination(at: index, progress: 0.37)
            )
        }
    }

    func testTransitionHasStableEndpoints() {
        let transition = SceneTransition(cellCount: 32, seed: 7)

        for index in 0..<32 {
            XCTAssertFalse(transition.usesDestination(at: index, progress: 0.0))
            XCTAssertTrue(transition.usesDestination(at: index, progress: 1.0))
        }
    }

    func testProgressIsClampedAndOutOfRangeIndexesAreSafe() {
        let transition = SceneTransition(cellCount: 1, seed: 7)

        XCTAssertFalse(transition.usesDestination(at: 0, progress: -1.0))
        XCTAssertTrue(transition.usesDestination(at: 0, progress: 2.0))
        XCTAssertFalse(transition.usesDestination(at: 10, progress: 0.5))
        XCTAssertTrue(transition.usesDestination(at: 10, progress: 1.0))
    }
}
