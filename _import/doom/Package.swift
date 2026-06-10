// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "DOOM",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "Hotword", targets: ["Hotword"]),
        .library(name: "PTYBridge", targets: ["PTYBridge"]),
        .executable(name: "Server", targets: ["Server"]),
        .executable(name: "WatcherCLI", targets: ["WatcherCLI"])
    ],
    dependencies: [
        .package(url: "https://github.com/vapor/vapor.git", from: "4.92.0")
    ],
    targets: [
        .target(
            name: "Hotword",
            dependencies: []
        ),
        .target(
            name: "PTYBridge",
            dependencies: [],
            cSettings: [
                .define("DARWIN", .when(platforms: [.macOS]))
            ]
        ),
        .executableTarget(
            name: "Server",
            dependencies: [
                .product(name: "Vapor", package: "vapor"),
                "PTYBridge"
            ],
            resources: [
                .process("Public")
            ]
        ),
        .executableTarget(
            name: "WatcherCLI",
            dependencies: [
                "Hotword"
            ]
        ),
        .testTarget(
            name: "HotwordTests",
            dependencies: ["Hotword"]
        ),
        .testTarget(
            name: "ServerTests",
            dependencies: [
                "Server",
                .product(name: "Vapor", package: "vapor")
            ]
        )
    ]
)
