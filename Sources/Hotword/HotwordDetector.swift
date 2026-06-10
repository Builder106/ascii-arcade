public struct HotwordDetector {
	private let pattern: [Character]
	private let isCaseSensitive: Bool
	private let windowMs: Double
	private var matchedCount: Int = 0
	private var lastTimestampMs: Double = .nan

	public init(hotword: String, isCaseSensitive: Bool = false, windowMs: Double = 1500) {
		self.isCaseSensitive = isCaseSensitive
		self.windowMs = windowMs
		self.pattern = isCaseSensitive ? Array(hotword) : Array(hotword.lowercased())
	}

	@discardableResult
	public mutating func push(_ input: Character, timestampMs: Double) -> Bool {
		let ch = isCaseSensitive ? input : Character(String(input).lowercased())

		if matchedCount > 0, timestampMs - lastTimestampMs > windowMs {
			matchedCount = 0
		}

		let expected = pattern[matchedCount]
		if ch == expected {
			matchedCount += 1
			lastTimestampMs = timestampMs
			if matchedCount == pattern.count {
				matchedCount = overlapCountAfterFullMatch(nextStartWith: ch)
				return true
			}
			return false
		}

		// Mismatch: consider KMP-like fallback for overlaps
		if matchedCount > 0 {
			matchedCount = fallbackOverlapCount(current: ch)
			if matchedCount == pattern.count {
				matchedCount = overlapCountAfterFullMatch(nextStartWith: ch)
				lastTimestampMs = timestampMs
				return true
			}
		}
		lastTimestampMs = timestampMs
		return false
	}

	private func overlapCountAfterFullMatch(nextStartWith ch: Character) -> Int {
		var count = 0
		for i in stride(from: pattern.count - 1, through: 1, by: -1) {
			if Array(pattern[0..<i]) == Array(pattern[(pattern.count - i)..<pattern.count]) {
				count = i
				break
			}
		}
		return count
	}

	private func fallbackOverlapCount(current ch: Character) -> Int {
		var newCount = 0
		for i in stride(from: matchedCount, through: 0, by: -1) {
			let prefix = Array(pattern[0..<i])
			let suffix = Array(pattern[(matchedCount - i)..<matchedCount])
			if prefix == suffix {
				newCount = i
				break
			}
		}
		if newCount < pattern.count, pattern[newCount] == ch {
			return newCount + 1
		}
		return newCount
	}
}
