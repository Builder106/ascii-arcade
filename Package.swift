// swift-tools-version:5.10
import PackageDescription

var targets: [Target] = [
    .target(name: "AsciiArcadeCore", dependencies: ["PTYBridge"], path: "Sources/AsciiArcadeCore"),
    .target(name: "PTYBridge", dependencies: [], path: "Sources/PTYBridge", cSettings: [
        .define("DARWIN", .when(platforms: [.macOS]))
    ]),
    .target(name: "Hotword", dependencies: [], path: "Sources/Hotword"),
]

var products: [Product] = [
    .library(name: "AsciiArcadeCore", targets: ["AsciiArcadeCore"]),
    .library(name: "PTYBridge", targets: ["PTYBridge"]),
    .library(name: "Hotword", targets: ["Hotword"]),
]

#if os(macOS)
targets.append(.executableTarget(name: "AsciiArcade", dependencies: ["AsciiArcadeCore", "Hotword"], path: "Sources/AsciiArcade"))
targets.append(.executableTarget(name: "WatcherCLI", dependencies: ["Hotword"], path: "Sources/WatcherCLI"))
products.append(.executable(name: "AsciiArcade", targets: ["AsciiArcade"]))
products.append(.executable(name: "WatcherCLI", targets: ["WatcherCLI"]))
#endif

targets += [
    .testTarget(name: "AsciiArcadeCoreTests", dependencies: ["AsciiArcadeCore"], path: "Tests/AsciiArcadeCoreTests"),
    .testTarget(name: "HotwordTests", dependencies: ["Hotword"], path: "Tests/HotwordTests"),
]

let package = Package(
    name: "AsciiArcade",
    platforms: [
        .macOS(.v13)
    ],
    products: products,
    targets: targets
)
