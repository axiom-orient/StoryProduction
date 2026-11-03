// swift-tools-version: 6.2
import PackageDescription

let package = Package(
  name: "StoryProductionSEMIProvider",
  platforms: [.macOS(.v26)],
  products: [
    .executable(name: "story-semi-provider", targets: ["StoryProductionSEMIProvider"])
  ],
  dependencies: [
    .package(path: "../../../SEMI-ControlPlane/Packages/SEMIProviderSDK"),
    .package(url: "https://github.com/swiftlang/swift-subprocess.git", exact: "1.0.0"),
  ],
  targets: [
    .executableTarget(
      name: "StoryProductionSEMIProvider",
      dependencies: [
        .product(name: "SEMICore", package: "SEMIProviderSDK"),
        .product(name: "SEMIControl", package: "SEMIProviderSDK"),
        .product(name: "SEMIMCP", package: "SEMIProviderSDK"),
        .product(name: "SEMIInterApp", package: "SEMIProviderSDK"),
        .product(name: "Subprocess", package: "swift-subprocess"),
      ],
      exclude: ["Info.plist"],
      linkerSettings: [
        .unsafeFlags(
          [
            "-Xlinker", "-sectcreate",
            "-Xlinker", "__TEXT",
            "-Xlinker", "__info_plist",
            "-Xlinker", "Sources/StoryProductionSEMIProvider/Info.plist",
          ], .when(platforms: [.macOS]))
      ]
    ),
    .testTarget(
      name: "StoryProductionSEMIProviderTests",
      dependencies: [
        "StoryProductionSEMIProvider",
        .product(name: "SEMICore", package: "SEMIProviderSDK"),
        .product(name: "SEMIControl", package: "SEMIProviderSDK"),
      ]
    ),
  ],
  swiftLanguageModes: [.v6]
)
