// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "dropbear",
    products: [
        .library(
            name: "dropbear",
            type: .dynamic,
            targets: ["dropbear"],
        ),
    ],
    dependencies: [
        // .package(url: "https://github.com/nicklockwood/VectorMath", from: "0.4.0")
    ],
    targets: [
        .target(
            name: "dropbear"
        ),
        .testTarget(
            name: "dropbearTests",
            dependencies: ["dropbear"]
        ),
    ]
)
