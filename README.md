# dropbear

dropbear is a game engine used to create games. It's made in rust. It's name is a double entendre, with it being the nickname of koalas but also fits in nicely with the theme of rust utilising memory management with "drops".

If you might have not realised, all the crates/projects names are after Australian flora and fauna.

## Related projects

- [eucalyptus](https://github.com/4tkbytes/dropbear/tree/main/eucalyptus) is the visual editor used to create games visually, taking inspiration from Unity, Roblox Studio and other engines.
- [redback](https://github.com/4tkbytes/redback-runtime) is the runtime used to load .eupak files and run the games loaded on them.

## Build

To build, ensure build requirements, clone the repository, then build it. It will build in debug mode, and use a lot of packages, so if your CPU is not fast enough for building you should brew a cup of coffee during the build time.

With Unix systems (macOS not tested), you will have to download a couple dependencies if building locally:

<!-- If you have a macOS system, please create a PR and add your own implementation. I know you need to use brew, but I don't know what dependencies to install.  -->

```bash
# ubuntu, adapt to your own OS
sudo apt install libudev-dev pkg-config libssl-dev clang cmake meson

# if on arm devices where russimp cannot compile
sudo apt install assimp-utils
```

After downloading the requirements, you are free to build it using cargo.

```bash
git clone git@github.com:4tkbytes/dropbear
cd dropbear

# ensure submodules are checked-out
git submodule init
git submodule update

# this will build all the projects in the workspace, including eucalyptus and redback.
cargo build
```

If you do not want to build it locally, you are able to download the latest action build (if no releases have been made).

[nightly.link](https://nightly.link/4tkbytes/dropbear/workflows/create_executable.yaml/main?preview)

## Usage

~~Depsite it looking like a dependency for `eucalyptus`, it can serve as a framework too. Looking through the `docs.rs` will you find related documentation onhow to use it and for rendering your own projects.~~

dropbear cannot be used as a framework (yet), but is best compatible with the eucalyptus editor when making games. For 
scripting, eucalyptus uses `rhai`, a new language that works with rust. 

The rhai reference for the eucalyptus editor is under the /docs folder of this repository, so take a look there. 
[Here is the entrance](https://github.com/4tkbytes/dropbear/blob/main/docs/README.md)

## Compability

|            | Windows | macOS | Linux | Web | Android | iOS |
|------------|---------|-------|-------|-----|---------|-----|
| eucalyptus |    ✅    |   ✅   |   ✅   |  ❌<sup>1</sup>  |    ❌<sup>1</sup>    |  ❌<sup>1</sup>  |
| redback    |    ✅    |   ✅   |   ✅   |  ❌<sup>2</sup>  |    ❌<sup>2</sup>    |  ❌  |

<sup>1</sup> Will never be implemented; not intended for that platform.

<sup>2</sup>  Made some progress on implementing.


To be fair, I do not plan on supporting web, android or iOS yet (as it isnt even completed with the basic idea). Maybe I will...?

## Contributions

Yeah yeah, go ahead and contribute. Make sure it works, and its not spam, and any tests pass.

# Licensing
In the case someone actually makes something with my engine and distributes it, it (meaning **dropbear-engine**, 
**eucalyptus** and **redback-runtime**) must abide by the license in [LICENSE.md](LICENSE.md). 

The gleek package is licensed under the [MIT License](https://mit-license.org/), which allows for anyone to use my 
library without _much_ restrictions. 