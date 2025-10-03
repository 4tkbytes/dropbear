# dropbear

dropbear is a game engine used to create games, made in Rust and scripted with the Kotlin Language.

It's name is a double entendre, with it being the nickname of koalas but also fits in nicely with the theme of rust utilising memory management with "drops".

If you might have not realised, all the crates/projects names are after Australian items.

## Projects

- [dropbear-engine](https://github.com/4tkbytes/dropbear/tree/main/dropbear-engine) is the rendering engine that uses wgpu and the main name of the project.
- [eucalyptus-editor](https://github.com/4tkbytes/dropbear/tree/main/eucalyptus-editor) is the visual editor used to create games visually, taking inspiration from Unity, Unreal, Roblox Studio and other engines.
- [eucalyptus-core](https://github.com/4tkbytes/dropbear/tree/main/eucalyptus-core) is the library used by both `redback-runtime` and `eucalyptus-editor` to share configs and metadata between each other.
- [redback-runtime](https://github.com/4tkbytes/redback-runtime) is the runtime used to load .eupak files and run the game loaded on them.

### Related Projects

- [dropbear_future-queue](https://github.com/4tkbytes/dropbear/tree/main/dropbear_future-queue) is a handy library for dealing with async in a sync context
- [model_to_image](https://github.com/4tkbytes/model_to_image) is a library used to generate thumbnails and images from a 3D model with the help of `russimp-ng` and a custom made rasteriser. _(very crude but usable)_

## Build

To build, ensure build requirements, clone the repository, then build it. It will build in debug mode, and use a lot of packages, so if your CPU is not fast enough for building you should brew a cup of coffee during the build time.

With Unix systems (macOS not tested), you will have to download a couple of dependencies if building locally:

<!-- If you have a macOS system, please create a PR and add your own implementation. I know you need to use brew, but I don't know what dependencies to install.  -->

### Dependencies

```bash
# ubuntu
sudo apt install libudev-dev pkg-config libssl-dev clang cmake meson assimp-utils openjdk-21-jdk

# i use arch btw
sudo pacman -Syu base-devel systemd pkgconf openssl clang cmake meson assimp jdk21-openjdk

```

### Engine Build

```bash
git clone git@github.com:4tkbytes/dropbear
cd dropbear

# this will build all the projects in the workspace
cargo build
```

[//]: # (# ensure submodules are checked-out)

[//]: # (git submodule init)

[//]: # (git submodule update)

> [!TIP]
> During the development of your game, you should download JetBrains IntelliJ (for kotlin) for the best support. 
> 
> For a development build of the eucalyptus editor, it is recommended that you use JetBrains IntelliJ (for jvm) and 
> JetBrains RustRover (for rust)

### Prebuilt

If you do not want to build it locally, you are able to download the latest action build (if no releases have been made).

[nightly.link](https://nightly.link/4tkbytes/dropbear/workflows/create_executable.yaml/main?preview)

## Usage

Despite the dropbear-engine (and other components) being made in Rust, the editor has chosen the scripting language of choice to be `Kotlin`
because of previous experience and that Kotlin is more multiplatform than Swift. 

Java is possible (as the JVM runs the script), however it is not officially supported and Kotlin is recommended. . 

API documentation and articles are available at (todo)

## Compability

|            | Windows | macOS | Linux | Web           | Android       | iOS           |
|------------|---------|-------|-------|---------------|---------------|---------------|
| eucalyptus | ✅       | ✅     | ✅     | ❌<sup>1</sup> | ❌<sup>1</sup> | ❌<sup>1</sup> |
| redback    | ✅       | ✅     | ✅     | ❌<sup>2</sup> | ❌<sup>2</sup> | ❌             |

<sup>1</sup> Will never be implemented; not intended for that platform.

<sup>2</sup>  Made some progress on implementing, but currently a WIP.

## Contributions

Yeah yeah, go ahead and contribute. Make sure it works, and its not spam, and any tests pass.

# Licensing

In the case someone actually makes something with my engine and distributes it, the projects (meaning **dropbear-engine**,
**eucalyptus** and **redback-runtime**) must abide by the license in [LICENSE.md](LICENSE.md).

The **dropbear_future-queue** rust library is available under the `MIT` license, which can be used by anyone.
