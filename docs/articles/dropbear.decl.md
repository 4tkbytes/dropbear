# What's the deal with com.dropbear.decl?

This article is to guide you on what the `com.dropbear.decl` package is for.

## What is it?

In the dropbear engine, there is a tool called `magna-carta` named after the document Magna Carta. `magna-carta` is a 
tool that generates Kotlin metadata for the engine to use. Since Kotlin/Native does not support reflection, or compile-time
KSP (Kotlin Symbol Processing), the engine uses a custom-made tool to generate metadata.

`magna-carta` essentially walks through your source directory and finds all classes with the annotation `@Runnable` using a 
tree-setter parser. It then generates Kotlin metadata for the classes and stores it in a file. The metadata is then used by
the engine to load the classes by its tag (for both the JVM and Native targets). Different targets
have different loading behaviours, which is why you would see `@CName("dropbear_load")` and `@CName("dropbear_update")`
and `@CName("dropbear_destroy")` in native generated code, but only see a ScriptRegistry in the JVM generated code.

## You still answer my question

Ah yes, my bad. When running on the JVM, the engine is required to provide some sort of path. It is hard to figure out
where the domain.package package may be, so it just generates in one centralised location: `com.dropbear.decl`.