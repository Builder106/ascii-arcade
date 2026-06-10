import Foundation

/// Where to find the `doom_ascii` binary + IWAD and how to launch it.
public struct DoomLaunchConfig {
    public let executablePath: String
    public let arguments: [String]
    public let environment: [String: String]
}

/// Resolves the vendored `doom_ascii` binary and a playable IWAD.
///
/// Ported from the old Vapor `routes.swift` resolver so both the wallpaper app
/// and the bonus browser server share one search policy.
public enum DoomLauncher {
    private static let iwadCandidates = [
        "doom.wad", "doom1.wad", "doom2.wad",
        "plutonia.wad", "tnt.wad",
        "chex.wad", "hacx.wad",
        "freedoom1.wad", "freedoom2.wad", "freedoom.wad", "freedm.wad"
    ]

    public static func resolve(
        workingDirectory: String = FileManager.default.currentDirectoryPath
    ) -> DoomLaunchConfig? {
        let fm = FileManager.default
        guard let bin = resolveBinary(fm: fm, workingDirectory: workingDirectory) else { return nil }

        var env = ProcessInfo.processInfo.environment
        var args = ["-chars", "block"]
        if let (iwad, dir) = resolveIwad(fm: fm, workingDirectory: workingDirectory) {
            args += ["-iwad", iwad]
            env["DOOMWADDIR"] = dir
        } else if let wad = env["DOOM_WAD_DIR"], !wad.isEmpty {
            env["DOOMWADDIR"] = wad
        }
        return DoomLaunchConfig(executablePath: bin, arguments: args, environment: env)
    }

    public static func resolveBinary(
        fm: FileManager = .default,
        workingDirectory: String = FileManager.default.currentDirectoryPath
    ) -> String? {
        let env = ProcessInfo.processInfo.environment
        if let e = env["DOOM_ASCII_PATH"], fm.isExecutableFile(atPath: e) { return e }
        let local = (workingDirectory as NSString).appendingPathComponent("bin/doom_ascii")
        if fm.isExecutableFile(atPath: local) { return local }
        for usr in ["/usr/local/bin/doom_ascii", "/opt/homebrew/bin/doom_ascii"] {
            if fm.isExecutableFile(atPath: usr) { return usr }
        }
        return nil
    }

    public static func resolveIwad(
        fm: FileManager = .default,
        workingDirectory: String = FileManager.default.currentDirectoryPath
    ) -> (iwadPath: String, dir: String)? {
        let env = ProcessInfo.processInfo.environment
        var searchDirs: [String] = []
        if let wadDir = env["DOOM_WAD_DIR"], !wadDir.isEmpty { searchDirs.append(wadDir) }
        searchDirs.append((workingDirectory as NSString).appendingPathComponent("wad"))
        searchDirs.append("/opt/homebrew/share/games/doom")
        searchDirs.append("/usr/local/share/games/doom")
        searchDirs.append("/usr/share/games/doom")

        for dir in searchDirs {
            for name in iwadCandidates {
                let full = (dir as NSString).appendingPathComponent(name)
                if fm.fileExists(atPath: full) { return (full, dir) }
            }
        }
        return nil
    }
}
