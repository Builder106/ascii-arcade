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
        .executable(name: "Server", targets: ["Server"]),
        .executable(name: "WatcherCLI", targets: ["WatcherCLI"])
    ],
    dependencies: [
        .package(url: "https://github.com/vapor/vapor.git", from: "4.92.0")
    ],
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
        // Bonus: stream DOOM to a browser tab over a Vapor WebSocket.
        .executableTarget(
            name: "Server",
            dependencies: [
                .product(name: "Vapor", package: "vapor"),
                "AsciiArcadeCore"
            ],
            path: "Sources/Server",
            resources: [
                .process("Public")
            ]
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
        ),
        .testTarget(
            name: "ServerTests",
            dependencies: [
                "Server",
                .product(name: "Vapor", package: "vapor")
            ],
            path: "Tests/ServerTests"
        )
    ]
)
