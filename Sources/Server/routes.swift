import Vapor
import Foundation
import PTYBridge

private func resolveDoomAsciiPath(req: Request) -> String? {
	let env = ProcessInfo.processInfo.environment
	if let e = env["DOOM_ASCII_PATH"], FileManager.default.isExecutableFile(atPath: e) { return e }
	let local = req.application.directory.workingDirectory + "bin/doom_ascii"
	if FileManager.default.isExecutableFile(atPath: local) { return local }
	let usr = "/usr/local/bin/doom_ascii"
	if FileManager.default.isExecutableFile(atPath: usr) { return usr }
	return nil
}

public func routes(_ app: Application) throws {
	app.get { req async in
		return req.fileio.streamFile(at: req.application.directory.publicDirectory + "index.html")
	}

	app.webSocket("ws", "doom") { req, ws in
		guard let path = resolveDoomAsciiPath(req: req) else {
			ws.send("doom_ascii not found. Run scripts/setup.sh to build it.")
			ws.close(promise: nil)
			return
		}
		var env = ProcessInfo.processInfo.environment
		if let wad = env["DOOM_WAD_DIR"] { env["DOOMWADDIR"] = wad }
		let builder = PTYProcessBuilder(
			launchPath: path,
			arguments: ["-chars", "block"],
			environment: env
		)
		let proc: PTYProcess
		do {
			proc = try builder.spawn(columns: 100, rows: 40)
		} catch {
			ws.close(promise: nil)
			return
		}

		proc.onOutput { data in
			if let s = String(data: data, encoding: .utf8) {
				ws.send(s)
			}
		}

		ws.onText { ws, text in
			proc.send(data: Data(text.utf8))
		}

		ws.onClose.whenComplete { _ in
			proc.terminate()
		}
	}
}
