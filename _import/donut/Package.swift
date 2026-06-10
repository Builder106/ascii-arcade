// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "DonutWallpaper",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "DonutCore", targets: ["DonutCore"]),
        .executable(name: "DonutWallpaperApp", targets: ["DonutWallpaperApp"])
    ],
    dependencies: [],
    targets: [
        .target(
            name: "DonutCore",
            path: "Sources/DonutCore"
        ),
        .executableTarget(
            name: "DonutWallpaperApp",
            dependencies: ["DonutCore"],
            path: "Sources/DonutWallpaperApp"
        ),
        .testTarget(
            name: "DonutCoreTests",
            dependencies: ["DonutCore"],
            path: "Tests/DonutCoreTests"
        )
    ]
)
