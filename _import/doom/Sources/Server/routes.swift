import Vapor
import Foundation
import PTYBridge
import NIOCore

private func resolveDoomAsciiPath(req: Request) -> String? {
	let env = ProcessInfo.processInfo.environment
	if let e = env["DOOM_ASCII_PATH"], FileManager.default.isExecutableFile(atPath: e) { return e }
	let local = req.application.directory.workingDirectory + "bin/doom_ascii"
	if FileManager.default.isExecutableFile(atPath: local) { return local }
	let usr = "/usr/local/bin/doom_ascii"
	if FileManager.default.isExecutableFile(atPath: usr) { return usr }
	return nil
}

private func resolveIwad(req: Request) -> (iwadPath: String, dirForEnv: String)? {
	let fm = FileManager.default
	let env = ProcessInfo.processInfo.environment

	var searchDirs: [String] = []
	if let wadDir = env["DOOM_WAD_DIR"], !wadDir.isEmpty { searchDirs.append(wadDir) }
	searchDirs.append(req.application.directory.workingDirectory + "wad")
	searchDirs.append("/opt/homebrew/share/games/doom")
	searchDirs.append("/usr/local/share/games/doom")
	searchDirs.append("/usr/share/games/doom")

	let iwadCandidates = [
		"doom.wad","doom1.wad","doom2.wad",
		"plutonia.wad","tnt.wad",
		"chex.wad","hacx.wad",
		"freedoom1.wad","freedoom2.wad","freedoom.wad","freedm.wad"
	]
	for dir in searchDirs {
		for name in iwadCandidates {
			let full = (dir as NSString).appendingPathComponent(name)
			if fm.fileExists(atPath: full) { return (full, dir) }
		}
	}
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
		if let wad = env["DOOM_WAD_DIR"], !wad.isEmpty { env["DOOMWADDIR"] = wad }

		var args: [String] = ["-chars","block"]
		if let (iwadPath, dir) = resolveIwad(req: req) {
			args += ["-iwad", iwadPath]
			env["DOOMWADDIR"] = dir
		}

		let builder = PTYProcessBuilder(
			launchPath: path,
			arguments: args,
			environment: env
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
