# magna-carta

Creates a manifest for kotlin files to allow for compile-time annotation processing. 

This crate was created because Kotlin Symbol Processor (KSP) does not support Kotlin Multiplatform (KMP) Native 
targets.

This crate also allows for easy *plugin* usage (italics because there is no plugins, just a bunch of traits), 
so you can have other languages (using the tree-setter parser), or saving the manifest for another target.

## Behaviour

### Common

magna-carta will look for all files in the `src/commonMain/kotlin` directory and generate a manifest file in 
`build/magna-carta/manifest.json`. This can be used for either [Native](#native) or [JVM](#jvm) targets, which
each have their own behavior. 

### Native

In the case the Native build is requested, it will generate a manifest Kotlin file in `src/nativeMain/kotlin`, which
includes exported C ABI entry points for dropbear to call into. 

### JVM

The JVM will also generate a manifest Kotlin file in `src/jvmMain/kotlin`, however because of its integration
with the `jni` crate, it will not generate any C ABI entry points, instead allowing for reflection to be used. 

# Usage

#### Generating for a JVM target:
```bash
magna-carta --input /home/tirbofish/project2/src --output /home/tirbofish/project2/build/magna-carta/jvmMain --target jvm
```

#### Generating for a Native target:
```bash
magna-carta --input /home/tirbofish/project2/src --output /home/tirbofish/project2/build/magna-carta/nativeLibMain --target native
```

#### Printing to stdout:
```bash
magna-carta --input /home/tirbofish/project2/src --target jvm --stdout
```

#### Getting raw manifest data (don't pipe this to a file, this is just for debugging):
```bash
magna-carta --input /home/tirbofish/project2/src --target jvm --stdout --raw
```