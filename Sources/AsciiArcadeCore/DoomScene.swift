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
    private var proc: PTYProcess?
    private var cols: Int
    private var rows: Int
    private var running = false
    private let stateLock = NSLock()

    public init(
        displayName: String = "DOOM",
        workingDirectory: String = FileManager.default.currentDirectoryPath,
        initialColumns: Int = 100,
        initialRows: Int = 40
    ) {
        self.displayName = displayName
        self.workingDirectory = workingDirectory
        self.cols = max(1, initialColumns)
        self.rows = max(1, initialRows)
        self.buffer = DoomScreenBuffer(width: self.cols, height: self.rows)
    }

    public func setGrid(width: Int, height: Int) {
        guard width > 0, height > 0 else { return }
        stateLock.lock()
        let changed = (width != cols || height != rows)
        cols = width; rows = height
        let p = proc
        stateLock.unlock()
        guard changed else { return }
        buffer.resize(width: width, height: height)
        p?.resize(columns: Int32(width), rows: Int32(height))
    }

    public func start() {
        stateLock.lock()
        if running { stateLock.unlock(); return }
        let (c, r) = (cols, rows)
        stateLock.unlock()

        guard let cfg = DoomLauncher.resolve(workingDirectory: workingDirectory) else {
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

    public func sendKey(_ bytes: [UInt8]) {
        stateLock.lock(); let p = proc; stateLock.unlock()
        guard !bytes.isEmpty else { return }
        p?.send(data: Data(bytes))
    }
}
