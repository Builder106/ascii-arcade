// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "AsciiArcade",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "AsciiArcadeCore", targets: ["AsciiArcadeCore"]),
        .library(name: "PTYBridge", targets: ["PTYBridge"]),
        .library(name: "Hotword", targets: ["Hotword"]),
        .executable(name: "AsciiArcade", targets: ["AsciiArcade"]),
        .executable(name: "WatcherCLI", targets: ["WatcherCLI"])
    ],
    // The Vapor-dependent "Server" bonus target (streams DOOM to a browser tab)
    // lives in its own package at Server/Package.swift — build/run it from
    // there (`swift run --package-path Server Server`) so a build of the
    // wallpaper package itself never resolves Vapor's dependency tree.
    targets: [
        // Frame generators (donut/helix), the unified scene protocol,
        // and the DOOM-as-a-scene glue (ANSI screen buffer + PTY-backed scene).
        .target(
            name: "AsciiArcadeCore",
            dependencies: ["PTYBridge"],
            path: "Sources/AsciiArcadeCore"
        ),
        .target(
            name: "PTYBridge",
            dependencies: [],
            path: "Sources/PTYBridge",
            cSettings: [
                .define("DARWIN", .when(platforms: [.macOS]))
            ]
        ),
        .target(
            name: "Hotword",
            dependencies: [],
            path: "Sources/Hotword"
        ),
        // The wallpaper host: picks a scene (donut / helix / DOOM) and a theme,
        // renders it to a desktop-level window, forwards keystrokes to DOOM.
        .executableTarget(
            name: "AsciiArcade",
            dependencies: ["AsciiArcadeCore", "Hotword"],
            path: "Sources/AsciiArcade"
        ),
        // Bonus: type the hotword anywhere to bring up the browser DOOM.
        .executableTarget(
            name: "WatcherCLI",
            dependencies: ["Hotword"],
            path: "Sources/WatcherCLI"
        ),
        .testTarget(
            name: "AsciiArcadeCoreTests",
            dependencies: ["AsciiArcadeCore"],
            path: "Tests/AsciiArcadeCoreTests"
        ),
        .testTarget(
            name: "HotwordTests",
            dependencies: ["Hotword"],
            path: "Tests/HotwordTests"
        )
    ]
)
