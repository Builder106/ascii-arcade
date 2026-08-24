#if os(macOS)
import Foundation
import PTYBridge

/// A playable DOOM cabinet: spawns `doom_ascii` in a PTY, reconstructs its
/// frames via `DoomScreenBuffer`, and forwards key bytes back to the process.
///
/// The PTY only runs while the scene is active (`start()`/`stop()`), so DOOM
/// isn't burning a CPU core when you're looking at the donut.
public final class DoomScene: AsciiScene {
    public let displayName: String
    public var isInteractive: Bool { true }

    private let buffer: DoomScreenBuffer
    private let workingDirectory: String
    private let scaling: Int
    private var proc: PTYProcess?
    private let cols: Int
    private let rows: Int
    private var running = false
    private let stateLock = NSLock()

    public init(
        displayName: String = "DOOM",
        workingDirectory: String = FileManager.default.currentDirectoryPath,
        scaling: Int? = nil
    ) {
        self.displayName = displayName
        self.workingDirectory = workingDirectory
        // Resolution lever: lower = sharper DOOM. 320/N × 200/N pixels. Default 1
        // renders DOOM's native 320×200 framebuffer — the sharpest the engine can
        // produce (~24fps as a wallpaper). Raise DOOM_SCALING for a lighter,
        // blockier frame (2 = 160×100 at ~30fps).
        let env = ProcessInfo.processInfo.environment
        let requested = scaling ?? env["DOOM_SCALING"].flatMap { Int($0) } ?? 1
        self.scaling = min(8, max(1, requested))
        let grid = DoomLauncher.gridSize(forScaling: self.scaling)
        self.cols = grid.cols
        self.rows = grid.rows
        self.buffer = DoomScreenBuffer(width: self.cols, height: self.rows)
    }

    /// DOOM drives its own fixed-resolution framebuffer, so the host paints it as
    /// a scaled colour bitmap rather than sizing it from the text font.
    public var fixedGrid: (width: Int, height: Int)? { (cols, rows) }

    /// No-op: DOOM's grid is pinned to its `-scaling` resolution, not the font.
    public func setGrid(width: Int, height: Int) {}

    public func start() {
        stateLock.lock()
        if running { stateLock.unlock(); return }
        let (c, r) = (cols, rows)
        stateLock.unlock()

        guard let cfg = DoomLauncher.resolve(workingDirectory: workingDirectory, scaling: scaling) else {
            buffer.showMessage("doom_ascii not found — run scripts/setup.sh")
            return
        }
        let builder = PTYProcessBuilder(
            launchPath: cfg.executablePath,
            arguments: cfg.arguments,
            environment: cfg.environment
        )
        do {
            let p = try builder.spawn(columns: Int32(c), rows: Int32(r))
            p.onOutput { [weak self] data in self?.buffer.feed(Array(data)) }
            stateLock.lock(); proc = p; running = true; stateLock.unlock()
        } catch {
            buffer.showMessage("Failed to launch DOOM")
        }
    }

    public func stop() {
        stateLock.lock()
        let p = proc
        proc = nil
        running = false
        stateLock.unlock()
        p?.terminate()
        buffer.clear()
    }

    public func frame(atTime t: Double) -> String {
        buffer.snapshot()
    }

    public func coloredFrame(atTime t: Double) -> ColoredFrame? {
        buffer.coloredSnapshot()
    }

    public func sendKey(_ bytes: [UInt8]) {
        stateLock.lock(); let p = proc; stateLock.unlock()
        guard !bytes.isEmpty else { return }
        p?.send(data: Data(bytes))
    }
}
#endif
