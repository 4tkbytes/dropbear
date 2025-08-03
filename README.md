# dropbear

dropbear is a game engine used to create games. It's made in rust. It's name is a double entendre, with it being the nickname of koalas but also fits in nicely with the theme of rust utilising "drops".

If you might have not realised, all the crates/projects names are after Australian flora and fauna.

## Related projects

- [eucalyptus](https://github.com/4tkbytes/dropbear/tree/main/eucalyptus) is the visual editor used to create games visually, taking inspiration from Unity and other engines.
- [redback](https://github.com/4tkbytes/dropbear/tree/main/redback) is the build system used by [eucalyptus](https://github.com/4tkbytes/dropbear/tree/main/eucalyptus) to bind, build and ship games made with the engine.

## Build

To build, ensure build requirements, clone the repository, then build it. It will build in debug mode, and use a lot of packages, so if your CPU is not fast enough for building you should brew a cup of coffee during the build time.

With Unix systems (macOS not tested), you will have to download a couple dependencies if building locally:
<!-- If you have a macOS system, please create a PR and add your own implementation. I know you need to use brew, but I don't know what dependencies to install.  -->

```bash
# ubuntu, adapt to your own OS
sudo apt install libudev-dev pkg-config libssl-dev clang

# if on arm devices where russimp cannot compile
sudo apt install assimp-utils
```

After downloading the requirements, you are free to build it using cargo.

```bash
git clone git@github.com:4tkbytes/dropbear
cd dropbear
# this will build all the projects in the workspace, including eucalyptus and redback.
cargo build
```

If you do not want to build it locally, you are able to download the latest action build (if no releases have been made).

[nightly.link](https://nightly.link/4tkbytes/dropbear/workflows/create_executable.yaml/main?preview)

## Usage

Depsite it looking like a dependency for `eucalyptus`, it can serve as a framework too. Looking through the `docs.rs` will you find related documentation onhow to use it and for rendering your own projects.

## Compability

|            | Windows | macOS | Linux | Web | Android | iOS |
|------------|---------|-------|-------|-----|---------|-----|
| dropbear   |    ✅    |   ✅   |   ✅   |  ❌  |    ❌    |  ❌  |
| eucalyptus |    ✅    |   ✅   |   ✅   |  ❌  |    ❌    |  ❌  |
| redback    |    ✅    |   ✅   |   ✅   |  ❌  |    ❌    |  ❌  |

To be fair, I do not plan on supporting web, android or iOS yet (as it isnt even completed with the basic idea). Maybe I will...?

## Contributions

Yeah yeah, go ahead and contribute. Make sure it works, and its not spam, and any tests pass. 
