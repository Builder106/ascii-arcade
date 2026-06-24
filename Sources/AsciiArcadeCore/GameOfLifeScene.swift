import Foundation

/// Conway's Game of Life on a toroidal (wrap-around) grid, seeded with classic
/// patterns — glider guns, spaceships, pulsars, methuselahs — rather than random
/// soup (which just decays into a scatter of tiny still-lifes). Cells are drawn
/// as solid blocks on a coarser logical grid scaled up, so the shapes are big
/// enough to read. Live cells are tinted by age (newborns flash bright, then
/// settle to the theme colour) and the board auto-reseeds when it stalls.
public final class GameOfLifeScene: SteppedScene {
    // Logical grid (coarser than the pixel grid, scaled up by `cellSize`).
    private var cols = 0
    private var rows = 0
    private var alive: [Bool] = []
    private var age: [Int] = []
    private var prev1: [Bool] = []      // 1 generation ago
    private var prev2: [Bool] = []      // 2 generations ago (catches period-2 oscillators)
    private var stableSteps = 0
    private var rng = SeededGenerator(seed: 0x11FE_C0DE)

    public init() {
        super.init(displayName: "Life")
    }

    public override var settings: [SceneSetting] {
        [
            SceneSetting(id: "speed", label: "Speed", options: [
                SceneOption(label: "Slow", value: 4),
                SceneOption(label: "Normal", value: 9),
                SceneOption(label: "Fast", value: 16),
            ], defaultIndex: 1),
            SceneSetting(id: "size", label: "Cell size", options: [
                SceneOption(label: "Small", value: 2),
                SceneOption(label: "Medium", value: 3),
                SceneOption(label: "Large", value: 4),
            ], defaultIndex: 1),
        ]
    }

    public override var stepInterval: Double {
        1.0 / max(1.0, settingValue("speed", default: 9))
    }

    private var cellSize: Int { max(1, Int(settingValue("size", default: 3))) }

    public override func reset() {
        recomputeLogical()
        seed()
    }

    public override func settingsChanged() {
        // A cell-size change resizes the logical grid → reseed; a speed change
        // only alters `stepInterval`, so leave the running board alone.
        let (oc, or) = (cols, rows)
        recomputeLogical()
        if cols != oc || rows != or { seed() }
    }

    public override func step() {
        let size = cols * rows
        guard size > 0, alive.count == size else { return }
        var next = Array(repeating: false, count: size)
        var nextAge = Array(repeating: 0, count: size)
        for y in 0..<rows {
            let yUp = (y - 1 + rows) % rows
            let yDn = (y + 1) % rows
            for x in 0..<cols {
                let xL = (x - 1 + cols) % cols
                let xR = (x + 1) % cols
                var n = 0
                if alive[yUp * cols + xL] { n += 1 }
                if alive[yUp * cols + x]  { n += 1 }
                if alive[yUp * cols + xR] { n += 1 }
                if alive[y * cols + xL]   { n += 1 }
                if alive[y * cols + xR]   { n += 1 }
                if alive[yDn * cols + xL] { n += 1 }
                if alive[yDn * cols + x]  { n += 1 }
                if alive[yDn * cols + xR] { n += 1 }
                let i = y * cols + x
                let live = alive[i] ? (n == 2 || n == 3) : (n == 3)
                next[i] = live
                if live { nextAge[i] = alive[i] ? min(age[i] + 1, 999) : 0 }
            }
        }

        // Reseed if the board emptied or settled into a fixed/period-2 pattern.
        let population = next.reduce(0) { $0 + ($1 ? 1 : 0) }
        if population == 0 { seed(); return }
        if next == prev1 || next == prev2 {
            stableSteps += 1
        } else {
            stableSteps = 0
        }
        prev2 = prev1
        prev1 = alive
        alive = next
        age = nextAge
        if stableSteps > 8 { seed() }
    }

    public override func render() -> ColoredFrame {
        let pixels = width * height
        var chars = Array(repeating: Character(" "), count: pixels)
        var colors = Array(repeating: RGBColor?.none, count: pixels)
        guard cols > 0, rows > 0, alive.count == cols * rows else {
            return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
        }
        let young = RGBColor.white.mixed(with: baseColor, t: 0.5)
        let s = cellSize
        for ly in 0..<rows {
            for lx in 0..<cols where alive[ly * cols + lx] {
                let color: RGBColor
                if age[ly * cols + lx] == 0 {
                    color = young
                } else {
                    color = baseColor.scaled(max(0.45, 1.0 - Double(age[ly * cols + lx]) * 0.05))
                }
                // Paint the cellSize × cellSize block this logical cell covers.
                for by in 0..<s {
                    let py = ly * s + by
                    if py >= height { break }
                    let rowBase = py * width
                    for bx in 0..<s {
                        let px = lx * s + bx
                        if px >= width { break }
                        chars[rowBase + px] = "█"
                        colors[rowBase + px] = color
                    }
                }
            }
        }
        return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
    }

    // MARK: - Seeding

    private func recomputeLogical() {
        cols = max(1, width / cellSize)
        rows = max(1, height / cellSize)
    }

    private func seed() {
        let size = cols * rows
        rng = SeededGenerator(seed: rng.next() ^ UInt64(cols &* 73856093 ^ rows &* 19349663))
        alive = Array(repeating: false, count: size)
        age = Array(repeating: 0, count: size)
        prev1 = alive
        prev2 = alive
        stableSteps = 0

        let area = size
        stamp(count: max(6, area / 180), from: [Patterns.glider])
        stamp(count: max(3, area / 450), from: [Patterns.lwss])
        var oscillators = [Patterns.blinker, Patterns.toad, Patterns.beacon]
        if cols >= 16 && rows >= 16 { oscillators.append(Patterns.pulsar) }
        stamp(count: max(2, area / 700), from: oscillators)
        stamp(count: max(1, area / 1400), from: [Patterns.acorn, Patterns.rPentomino])
        if cols >= 40 && rows >= 12 {
            stamp(count: min(2, 1 + area / 2500), from: [Patterns.gosperGun])
        }
    }

    private func stamp(count: Int, from pool: [[(Int, Int)]]) {
        for _ in 0..<count {
            let pattern = oriented(pool[Int.random(in: 0..<pool.count, using: &rng)])
            let ox = Int.random(in: 0..<cols, using: &rng)
            let oy = Int.random(in: 0..<rows, using: &rng)
            for (x, y) in pattern {
                let gx = (((ox + x) % cols) + cols) % cols
                let gy = (((oy + y) % rows) + rows) % rows
                alive[gy * cols + gx] = true
            }
        }
    }

    /// Randomly rotate (0/90/180/270) and optionally mirror a pattern, then
    /// normalise it to the origin, so the same template appears in many guises.
    private func oriented(_ cells: [(Int, Int)]) -> [(Int, Int)] {
        var out = cells
        if Bool.random(using: &rng) { out = out.map { (-$0.0, $0.1) } }
        for _ in 0..<Int.random(in: 0..<4, using: &rng) { out = out.map { (-$0.1, $0.0) } }
        let minX = out.map { $0.0 }.min() ?? 0
        let minY = out.map { $0.1 }.min() ?? 0
        return out.map { ($0.0 - minX, $0.1 - minY) }
    }
}

/// Classic Life patterns as `(x, y)` cell offsets.
private enum Patterns {
    static let glider: [(Int, Int)] = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]
    static let blinker: [(Int, Int)] = [(0, 0), (1, 0), (2, 0)]
    static let toad: [(Int, Int)] = [(1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)]
    static let beacon: [(Int, Int)] = [(0, 0), (1, 0), (0, 1), (1, 1), (2, 2), (3, 2), (2, 3), (3, 3)]
    static let rPentomino: [(Int, Int)] = [(1, 0), (2, 0), (0, 1), (1, 1), (1, 2)]
    static let acorn: [(Int, Int)] = [(1, 0), (3, 1), (0, 2), (1, 2), (4, 2), (5, 2), (6, 2)]

    /// Lightweight spaceship (travels across the torus).
    static let lwss: [(Int, Int)] = [
        (0, 0), (3, 0),
        (4, 1),
        (0, 2), (4, 2),
        (1, 3), (2, 3), (3, 3), (4, 3),
    ]

    /// Period-3 pulsar — a big, instantly recognisable flashing shape.
    static let pulsar: [(Int, Int)] = {
        let bars = [2, 3, 4, 8, 9, 10]
        let posts = [0, 5, 7, 12]
        var cells: [(Int, Int)] = []
        for x in bars { cells.append((x, 0)); cells.append((x, 5)); cells.append((x, 7)); cells.append((x, 12)) }
        for y in [2, 3, 4, 8, 9, 10] { for x in posts { cells.append((x, y)) } }
        return cells
    }()

    /// Gosper glider gun — continuously emits gliders.
    static let gosperGun: [(Int, Int)] = [
        (0, 4), (0, 5), (1, 4), (1, 5),
        (10, 4), (10, 5), (10, 6), (11, 3), (11, 7), (12, 2), (12, 8), (13, 2), (13, 8),
        (14, 5), (15, 3), (15, 7), (16, 4), (16, 5), (16, 6), (17, 5),
        (20, 2), (20, 3), (20, 4), (21, 2), (21, 3), (21, 4), (22, 1), (22, 5),
        (24, 0), (24, 1), (24, 5), (24, 6),
        (34, 2), (34, 3), (35, 2), (35, 3),
    ]
}
