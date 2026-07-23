// swift-tools-version:5.10
import PackageDescription

// Bonus target: streams DOOM to a browser tab over a Vapor WebSocket. Split
// into its own package so a from-scratch `swift build`/resolve of the root
// AsciiArcade wallpaper package doesn't also fetch and build Vapor's
// dependency tree — the wallpaper executable itself never depends on Vapor.
let package = Package(
    name: "AsciiArcadeServer",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(name: "Server", targets: ["Server"])
    ],
    dependencies: [
        .package(path: ".."),
        .package(url: "https://github.com/vapor/vapor.git", from: "4.92.0")
    ],
    targets: [
        .executableTarget(
            name: "Server",
            dependencies: [
                .product(name: "Vapor", package: "vapor"),
                .product(name: "AsciiArcadeCore", package: "ascii-arcade"),
                .product(name: "PTYBridge", package: "ascii-arcade")
            ],
            path: "Sources/Server",
            resources: [
                .process("Public")
            ]
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
