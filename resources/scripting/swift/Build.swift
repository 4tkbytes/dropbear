// swift-tools-version: 6.2

// Build system file for the dropbear-engine scripting API

import PackageDescription

let package = Package(
    name: "skibidi_toilet_goon_maxx",
    products: [
        .library(
            name: "skibidi_toilet_goon_maxx",
            type: .dynamic, // required as it will be linked to the final executable
            targets: ["skibidi_toilet_goon_maxx"]
        ),
    ],
    dependencies: [
        // add your own dependencies here!
        // .package(url: "https://github.com/4tkbytes/dropbear", from: "0.0.0")
    ],
    targets: [
        .target(
            name: "skibidi_toilet_goon_maxx",
            path: "src",
            exclude: [
                // exclude other items here
                "source.eucc"
            ]
        ),
    ]
)
