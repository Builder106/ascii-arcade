import Foundation

#if os(macOS)
import Darwin

public final class PTYProcess {
	private var masterFD: Int32 = -1
	private var childPID: pid_t = 0
	private let queue = DispatchQueue(label: "pty.read")
	private var outputHandler: ((Data) -> Void)?
	private var readerSource: DispatchSourceRead?

	public func onOutput(_ handler: @escaping (Data) -> Void) {
		self.outputHandler = handler
	}

	public func send(data: Data) {
		_ = data.withUnsafeBytes { ptr in
			write(masterFD, ptr.baseAddress, ptr.count)
		}
	}

	public func terminate() {
		if childPID > 0 {
			kill(childPID, SIGTERM)
		}
		if masterFD >= 0 {
			close(masterFD)
		}
		readerSource?.cancel()
	}

	fileprivate init(masterFD: Int32, pid: pid_t) {
		self.masterFD = masterFD
		self.childPID = pid
		let source = DispatchSource.makeReadSource(fileDescriptor: masterFD, queue: queue)
		source.setEventHandler { [weak self] in
			guard let self = self else { return }
			var buffer = [UInt8](repeating: 0, count: 8192)
			let n = read(self.masterFD, &buffer, buffer.count)
			if n > 0 {
				self.outputHandler?(Data(buffer[0..<n]))
			} else {
				self.readerSource?.cancel()
			}
		}
		source.resume()
		self.readerSource = source
	}
}

public struct PTYProcessBuilder {
	public let launchPath: String
	public let arguments: [String]
	public let environment: [String: String]

	public init(launchPath: String, arguments: [String], environment: [String: String]) {
		self.launchPath = launchPath
		self.arguments = arguments
		self.environment = environment
	}

	public func spawn(columns: Int32, rows: Int32) throws -> PTYProcess {
		var win = winsize(ws_row: UInt16(rows), ws_col: UInt16(columns), ws_xpixel: 0, ws_ypixel: 0)
		var master: Int32 = -1
		var pid = pid_t(0)
		pid = forkpty(&master, nil, nil, &win)
		guard pid >= 0 else { throw NSError(domain: "pty", code: 10) }
		if pid == 0 {
			var cArgs: [UnsafeMutablePointer<CChar>?] = [strdup(launchPath)]
			for a in arguments { cArgs.append(strdup(a)) }
			cArgs.append(nil)
			var cEnv: [UnsafeMutablePointer<CChar>?] = []
			for (k, v) in environment { cEnv.append(strdup("\(k)=\(v)")) }
			cEnv.append(nil)
			execve(launchPath, &cArgs, &cEnv)
			exit(127)
		}
		return PTYProcess(masterFD: master, pid: pid)
	}
}
#endif
