import Foundation

/// A deterministic, character-level handoff between two scene frames.
///
/// Each cell gets a stable threshold. As progress moves from zero to one, the
/// destination scene takes over cell by cell. Keeping the thresholds stable
/// makes an interrupted transition continue as a legible dissolve instead of
/// reshuffling the pattern on every redraw.
public struct SceneTransition: Sendable {
    public let cellCount: Int
    private let thresholds: [Double]

    public init(cellCount: Int, seed: UInt64 = 0x9E37_79B9_7F4A_7C15) {
        let count = max(0, cellCount)
        self.cellCount = count

        var state = seed == 0 ? 0x9E37_79B9_7F4A_7C15 : seed
        var values: [Double] = []
        values.reserveCapacity(count)
        for _ in 0..<count {
            // xorshift64* keeps this helper dependency-free and deterministic
            // across repeated runs.
            state ^= state >> 12
            state ^= state << 25
            state ^= state >> 27
            let mixed = state &* 0x2545_F491_4F6C_DD1D
            values.append(Double(mixed) / Double(UInt64.max))
        }
        thresholds = values
    }

    /// Whether the destination cell should be painted at `progress` (0...1).
    public func usesDestination(at index: Int, progress: Double) -> Bool {
        guard index >= 0, index < thresholds.count else { return progress >= 1.0 }
        let p = min(max(progress, 0.0), 1.0)
        return p >= 1.0 || (p > 0.0 && p > thresholds[index])
    }
}
