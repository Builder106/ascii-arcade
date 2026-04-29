import Foundation
import ApplicationServices
import Hotword

final class ServerManager {
	private var serverProcess: Process?

	private var baseURLString: String {
		let port = ProcessInfo.processInfo.environment["DOOM_PORT"] ?? ProcessInfo.processInfo.environment["PORT"] ?? "8787"
		return "http://127.0.0.1:\(port)/"
	}

	func ensureServerRunning() {
		if isServerUp() { return }
		launchServer()
	}

	private func isServerUp() -> Bool {
		let semaphore = DispatchSemaphore(value: 0)
		var ok = false
		guard let url = URL(string: baseURLString) else { return false }
		var request = URLRequest(url: url)
		request.timeoutInterval = 0.5
		let task = URLSession.shared.dataTask(with: request) { _, response, _ in
			if let http = response as? HTTPURLResponse { ok = (200...399).contains(http.statusCode) }
			semaphore.signal()
		}
		task.resume()
		_ = semaphore.wait(timeout: .now() + 1)
		return ok
	}

	private func launchServer() {
		let fm = FileManager.default
		let candidates = ["/usr/local/bin/doom-server", fm.currentDirectoryPath + "/.build/release/Server", fm.currentDirectoryPath + "/.build/debug/Server"]
		guard let path = candidates.first(where: { fm.isExecutableFile(atPath: $0) }) else { return }
		var env = ProcessInfo.processInfo.environment
		if env["DOOM_PORT"] == nil && env["PORT"] == nil {
			env["DOOM_PORT"] = "8787"
		}
		let p = Process()
		p.executableURL = URL(fileURLWithPath: path)
		p.standardOutput = FileHandle.nullDevice
		p.standardError = FileHandle.nullDevice
		p.environment = env
		try? p.run()
		serverProcess = p
	}

	func showWallpaper() {
		let fm = FileManager.default
		let plashApp = "/Applications/Plash.app"
		let urlString = baseURLString
		if fm.fileExists(atPath: plashApp) {
			let proc = Process()
			proc.executableURL = URL(fileURLWithPath: "/usr/bin/open")
			proc.arguments = ["-g", "plash:///add?url=\(urlString.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? urlString)"]
			try? proc.run()
			return
		}
		let proc = Process()
		proc.executableURL = URL(fileURLWithPath: "/usr/bin/open")
		proc.arguments = ["-g", urlString]
		try? proc.run()
	}
}

final class KeyboardMonitor {
	private var detector: HotwordDetector
	private let server = ServerManager()
	private var eventTap: CFMachPort?
	private var runLoopSource: CFRunLoopSource?

	init(hotword: String) {
		self.detector = HotwordDetector(hotword: hotword, isCaseSensitive: false, windowMs: 3000)
	}

	func start() {
		let mask = (1 << CGEventType.keyDown.rawValue)
		let callback: CGEventTapCallBack = { proxy, type, event, refcon in
			guard type == .keyDown else { return Unmanaged.passUnretained(event) }
			let monitor = Unmanaged<KeyboardMonitor>.fromOpaque(refcon!).takeUnretainedValue()
			var length = 0
			event.keyboardGetUnicodeString(maxStringLength: 8, actualStringLength: &length, unicodeString: nil)
			var buffer = [UniChar](repeating: 0, count: max(1, length))
			event.keyboardGetUnicodeString(maxStringLength: buffer.count, actualStringLength: &length, unicodeString: &buffer)
			let s = String(utf16CodeUnits: buffer, count: length)
			let now = Date().timeIntervalSince1970 * 1000
			for ch in s {
				if monitor.detector.push(ch, timestampMs: now) {
					DispatchQueue.global().async {
						monitor.server.ensureServerRunning()
						monitor.server.showWallpaper()
					}
				}
			}
			return Unmanaged.passUnretained(event)
		}
		let refcon = Unmanaged.passUnretained(self).toOpaque()
		guard let tap = CGEvent.tapCreate(tap: .cgSessionEventTap, place: .headInsertEventTap, options: .defaultTap, eventsOfInterest: CGEventMask(mask), callback: callback, userInfo: refcon) else { return }
		eventTap = tap
		runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
		CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
		CGEvent.tapEnable(tap: tap, enable: true)
		CFRunLoopRun()
	}
}

let monitor = KeyboardMonitor(hotword: "doom")
monitor.start()
