import Foundation

/// Reconstructs `doom_ascii`'s ANSI terminal stream into a fixed character grid.
///
/// `doom_ascii` (`-chars block`) redraws every frame as: cursor-home (`ESC[;H`),
/// an optional clear (`ESC[2J`), a bold flag, then per-pixel truecolor SGR codes
/// (`ESC[38;2;R;G;Bm`) followed by a block glyph, ending with a reset. We only
/// need the glyph grid for a themed monochrome wallpaper, so this buffer honors
/// cursor positioning + clears and discards the SGR colour/style codes.
///
/// Thread-safe: the PTY reader feeds bytes off the main thread while the view
/// snapshots on the main thread.
public final class DoomScreenBuffer {
    private var width: Int
    private var height: Int
    private var grid: [[Character]]
    private var cursorRow = 0
    private var cursorCol = 0
    private var pending: [UInt8] = []
    private let lock = NSLock()

    public init(width: Int, height: Int) {
        self.width = max(1, width)
        self.height = max(1, height)
        self.grid = Self.blankGrid(width: self.width, height: self.height)
    }

    // MARK: - Public API

    public func resize(width: Int, height: Int) {
        lock.lock(); defer { lock.unlock() }
        self.width = max(1, width)
        self.height = max(1, height)
        grid = Self.blankGrid(width: self.width, height: self.height)
        cursorRow = 0; cursorCol = 0
        pending.removeAll(keepingCapacity: true)
    }

    public func clear() {
        lock.lock(); defer { lock.unlock() }
        grid = Self.blankGrid(width: width, height: height)
        cursorRow = 0; cursorCol = 0
    }

    /// Render an informational line into an otherwise-blank grid (e.g. an error).
    public func showMessage(_ message: String) {
        lock.lock(); defer { lock.unlock() }
        grid = Self.blankGrid(width: width, height: height)
        let row = height / 2
        let chars = Array(message)
        let start = max(0, (width - chars.count) / 2)
        for (i, ch) in chars.enumerated() {
            let col = start + i
            if col < width { grid[row][col] = ch }
        }
        cursorRow = 0; cursorCol = 0
    }

    /// The current frame as `height` rows joined by newlines, each `width` wide.
    public func snapshot() -> String {
        lock.lock(); defer { lock.unlock() }
        var out = ""
        out.reserveCapacity((width + 1) * height)
        for row in 0..<height {
            out.append(String(grid[row]))
            if row < height - 1 { out.append("\n") }
        }
        return out
    }

    /// Feed raw PTY bytes. Incomplete escape sequences / multibyte glyphs that
    /// straddle a chunk boundary are stashed and resumed on the next feed.
    public func feed(_ bytes: [UInt8]) {
        lock.lock(); defer { lock.unlock() }
        var buf = pending
        buf.append(contentsOf: bytes)
        pending.removeAll(keepingCapacity: true)

        let count = buf.count
        var i = 0
        while i < count {
            let b = buf[i]
            switch b {
            case 0x1b: // ESC
                guard i + 1 < count else { stash(buf, from: i); return }
                let n = buf[i + 1]
                if n == 0x5b { // '[' → CSI
                    guard let end = scanCSITerminator(buf, start: i + 2, count: count) else {
                        stash(buf, from: i); return
                    }
                    applyCSI(params: Array(buf[(i + 2)..<end]), final: buf[end])
                    i = end + 1
                } else if n == 0x5d { // ']' → OSC, ends at BEL or ST (ESC \)
                    guard let end = scanOSCTerminator(buf, start: i + 2, count: count) else {
                        stash(buf, from: i); return
                    }
                    i = end
                } else {
                    // Two-byte escape (e.g. ESC \). Consume both.
                    i += 2
                }
            case 0x0a: // LF — treat as CR+LF (doom relies on EOL conversion)
                cursorRow += 1
                cursorCol = 0
                i += 1
            case 0x0d: // CR
                cursorCol = 0
                i += 1
            case 0x00..<0x20: // other control bytes — ignore
                i += 1
            default:
                if b < 0x80 {
                    place(Character(UnicodeScalar(b)))
                    i += 1
                } else {
                    let len = utf8Length(b)
                    guard i + len <= count else { stash(buf, from: i); return }
                    if let s = String(bytes: buf[i..<(i + len)], encoding: .utf8), let ch = s.first {
                        place(ch)
                    }
                    i += len
                }
            }
        }
    }

    // MARK: - Internals

    private func stash(_ buf: [UInt8], from index: Int) {
        pending = Array(buf[index...])
    }

    private func place(_ ch: Character) {
        guard cursorRow >= 0, cursorRow < height else { cursorCol += 1; return }
        if cursorCol >= 0, cursorCol < width {
            grid[cursorRow][cursorCol] = ch
        }
        cursorCol += 1
    }

    /// Finds the final byte (0x40–0x7e) of a CSI sequence; nil if not yet present.
    private func scanCSITerminator(_ buf: [UInt8], start: Int, count: Int) -> Int? {
        var j = start
        while j < count {
            let c = buf[j]
            if c >= 0x40 && c <= 0x7e { return j }
            j += 1
        }
        return nil
    }

    /// Finds the end index (one past the terminator) of an OSC sequence.
    private func scanOSCTerminator(_ buf: [UInt8], start: Int, count: Int) -> Int? {
        var j = start
        while j < count {
            if buf[j] == 0x07 { return j + 1 } // BEL
            if buf[j] == 0x1b, j + 1 < count, buf[j + 1] == 0x5c { return j + 2 } // ST = ESC \
            if buf[j] == 0x1b, j + 1 >= count { return nil } // possible partial ST
            j += 1
        }
        return nil
    }

    private func applyCSI(params: [UInt8], final: UInt8) {
        switch final {
        case 0x48, 0x66: // 'H' / 'f' → cursor position (1-based, default 1;1)
            let parts = String(decoding: params, as: UTF8.self).split(separator: ";", omittingEmptySubsequences: false)
            let row1 = parts.count > 0 ? Int(parts[0]) ?? 1 : 1
            let col1 = parts.count > 1 ? Int(parts[1]) ?? 1 : 1
            cursorRow = clamp(row1 - 1, 0, height - 1)
            cursorCol = clamp(col1 - 1, 0, max(0, width - 1))
        case 0x4a: // 'J' → erase display (treat any mode as full clear)
            grid = Self.blankGrid(width: width, height: height)
        case 0x4b: // 'K' → erase in line
            let mode = Int(String(decoding: params, as: UTF8.self)) ?? 0
            eraseLine(mode: mode)
        default:
            break // SGR ('m') and everything else: ignored
        }
    }

    private func eraseLine(mode: Int) {
        guard cursorRow >= 0, cursorRow < height else { return }
        switch mode {
        case 1: // start of line to cursor
            for c in 0...min(cursorCol, width - 1) where c >= 0 { grid[cursorRow][c] = " " }
        case 2: // whole line
            for c in 0..<width { grid[cursorRow][c] = " " }
        default: // cursor to end of line
            if cursorCol < width {
                for c in max(0, cursorCol)..<width { grid[cursorRow][c] = " " }
            }
        }
    }

    private func utf8Length(_ lead: UInt8) -> Int {
        if lead & 0xE0 == 0xC0 { return 2 }
        if lead & 0xF0 == 0xE0 { return 3 }
        if lead & 0xF8 == 0xF0 { return 4 }
        return 1
    }

    private func clamp(_ v: Int, _ lo: Int, _ hi: Int) -> Int { min(max(v, lo), hi) }

    private static func blankGrid(width: Int, height: Int) -> [[Character]] {
        Array(repeating: Array(repeating: Character(" "), count: width), count: height)
    }
}
