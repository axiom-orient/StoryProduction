// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "SceneMotionCompilerLauncher",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "SceneMotionCompilerLauncher", targets: ["SceneMotionCompilerLauncher"]),
        .library(name: "LauncherCore", targets: ["LauncherCore"])
    ],
    targets: [
        .target(name: "LauncherCore"),
        .executableTarget(name: "SceneMotionCompilerLauncher", dependencies: ["LauncherCore"]),
        .testTarget(name: "LauncherCoreTests", dependencies: ["LauncherCore"])
    ]
)
