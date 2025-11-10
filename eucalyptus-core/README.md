# eucalyptus-core

The core libraries of the eucalyptus editor. Great for embedding into `redback-runtime` and `eucalyptus-editor` as one big change instead of a bunch of features.

This is a library, so if tools are wished to be made, this is the perfect library for you.

it also produces a shared library for Kotlin/Native and the JVM :)

## Features

- `editor` - Enables editor only features that the redback-runtime would not be able to access
- `jvm` - Enables the JVM as a ScriptTarget and running the Java Virtual Machine (not possible with non-desktop targets)
- `jvm_debug` - Enables debugging of the JVM through the java debugger. Can pose a risk to tampering, so is disabled by default unless
    want to be enabled by developer or enabled by default by the `editor` feature. 