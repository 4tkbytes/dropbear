# notes to self

## hybrid approach:

- during dev builds, interpret using the JVM
- on production build, build to native.

to check if its prod or dev, we use a config to build.

workflow:
- user creates a new project, which loads the project
- use jvm to watch (because editor is always on desktop)
- on production build, create native build for specific OS

## required dependencies

- jdk 21
