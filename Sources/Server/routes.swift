import Vapor
import Foundation
import PTYBridge
import AsciiArcadeCore
import NIOCore

public func routes(_ app: Application) throws {
	app.get { req async in
		return req.fileio.streamFile(at: req.application.directory.publicDirectory + "index.html")
	}

	app.webSocket("ws", "doom") { req, ws in
		// Share the wallpaper app's binary/IWAD resolution policy.
		guard let cfg = DoomLauncher.resolve(workingDirectory: req.application.directory.workingDirectory) else {
			ws.send("doom_ascii not found. Run scripts/setup.sh to build it.")
			ws.close(promise: nil)
			return
		}

		let builder = PTYProcessBuilder(
			launchPath: cfg.executablePath,
			arguments: cfg.arguments,
			environment: cfg.environment
		)
		let proc: PTYProcess
		do {
			proc = try builder.spawn(columns: 100, rows: 40)
		} catch {
			ws.close(promise: nil)
			return
		}

		let allocator = ByteBufferAllocator()
		proc.onOutput { data in
			var buf = allocator.buffer(capacity: data.count)
			buf.writeBytes(data)
			ws.send(raw: buf.readableBytesView, opcode: .binary)
		}

		ws.onText { ws, text in
			if text.hasPrefix("__resize__:") {
				let json = String(text.dropFirst("__resize__:".count))
				if let data = json.data(using: .utf8), let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
					let cols = (obj["cols"] as? Int) ?? 100
					let rows = (obj["rows"] as? Int) ?? 40
					proc.resize(columns: Int32(cols), rows: Int32(rows))
				}
				return
			}
			proc.send(data: Data(text.utf8))
		}

		ws.onClose.whenComplete { _ in
			proc.terminate()
		}
	}
}
