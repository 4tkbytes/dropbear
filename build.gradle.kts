plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.kotlinxSerialization)
    `maven-publish`
    id("org.jetbrains.dokka") version "2.0.0"
}

group = "com.dropbear"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

val hostOs = providers.systemProperty("os.name").get()
val isArm64 = providers.systemProperty("os.arch").map { it == "aarch64" }.get()
val isMingwX64 = hostOs.startsWith("Windows")
val isLinux = hostOs == "Linux"
val isMacOs = hostOs == "Mac OS X"

val libName = when {
    isMacOs -> "libeucalyptus_core.dylib"
    isLinux -> "libeucalyptus_core.so"
    isMingwX64 -> "eucalyptus_core.dll"
    else -> throw GradleException("Host OS is not supported in Kotlin/Native.")
}

val libPathProvider = provider {
    val candidates = listOf(
        layout.projectDirectory.file("target/debug/$libName").asFile,
        layout.projectDirectory.file("target/release/$libName").asFile,
        layout.projectDirectory.file("libs/$libName").asFile
    )

    val foundFile = candidates.firstOrNull { it.exists() }
    if (foundFile != null) {
        foundFile.absolutePath
    } else {
        println("No Rust library exists")
        ""
    }
}

kotlin {
    jvm {
        withJava()
    }

    val nativeTarget = when {
        isMacOs && isArm64 -> macosArm64("nativeLib")
        isMacOs && !isArm64 -> macosX64("nativeLib")
        isLinux && isArm64 -> linuxArm64("nativeLib")
        isLinux && !isArm64 -> linuxX64("nativeLib")
        isMingwX64 -> mingwX64("nativeLib")
        else -> throw GradleException("Host OS is not supported in Kotlin/Native.")
    }

    val nativeLibPath = libPathProvider.get()
    val nativeLibDir = file(nativeLibPath).parentFile.absolutePath
    val nativeLibFileName = file(nativeLibPath).name
    val nativeLibNameForLinking = when {
        isMacOs -> nativeLibFileName.removePrefix("lib").removeSuffix(".dylib")
        isLinux -> nativeLibFileName.removePrefix("lib").removeSuffix(".so")
        isMingwX64 -> nativeLibFileName.removeSuffix(".dll")
        else -> throw GradleException("Unsupported OS for library name derivation.")
    }

    nativeTarget.apply {
        compilations.getByName("main") {
            cinterops {
                val dropbear by creating {
                    defFile(project.file("src/dropbear.def"))
                    includeDirs.headerFilterOnly(project.file("headers"))
                }
            }
        }
        binaries {
            sharedLib {
                baseName = "dropbear"

                if (isLinux || isMacOs) {
                    linkerOpts("-L$nativeLibDir", "-l$nativeLibNameForLinking", "-Wl,-rpath,\\\$ORIGIN")
                } else if (isMingwX64) {
                    val importLibName = "$nativeLibNameForLinking.dll.lib"
                    val importLibPath = file("$nativeLibDir/$importLibName").absolutePath
                    linkerOpts(
                        importLibPath
                    )
                }
            }
        }
    }

    sourceSets {
        commonMain {
            dependencies {
                implementation("org.jetbrains.kotlinx:kotlinx-datetime:0.6.0")
            }
        }
        nativeMain {
            dependencies {
                implementation(libs.kotlinxSerializationJson)
            }
        }

        jvmMain {
            kotlin.srcDirs("src/jvmMain/kotlin", "build/magna-carta")
            dependencies {

            }
        }
    }

    targets.all {
        compilations.all {
            compileTaskProvider.configure {
                compilerOptions {
                    freeCompilerArgs.add("-Xexpect-actual-classes")
                }
            }
        }
    }
}

tasks.register<JavaCompile>("generateJniHeaders") {
    val outputDir = layout.buildDirectory.dir("generated/jni-headers")
    options.headerOutputDirectory.set(outputDir.get().asFile)

    destinationDirectory.set(layout.buildDirectory.dir("classes/java/jni"))

    classpath = files(
        tasks.named("compileKotlinJvm"),
    )

    source = fileTree("src/jvmMain/java") {
        include("**/*.java")
    }

    dependsOn("compileKotlinJvm")
}

publishing {
    repositories {
        maven {
            name = "GitHubPages"
            url = uri(layout.buildDirectory.dir("repo"))
        }
    }

    publications.withType<MavenPublication> {
        pom {
            name.set("dropbear")
            description.set("The dropbear scripting part of the engine... uhh yeah!")
            url.set("https://github.com/4tkbytes/dropbear")

            licenses {
                license {
                    name.set("dropbear engine License, Version 1.2")
                    url.set("https://raw.githubusercontent.com/4tkbytes/dropbear/refs/heads/main/LICENSE.md")
                }
            }

            developers {
                developer {
                    id.set("4tkbytes")
                    name.set("tk")
                    email.set("4tkbytes@pm.me")
                }
            }

            scm {
                url.set("https://github.com/4tkbytes/dropbear")
                connection.set("scm:git:git://github.com/4tkbytes/dropbear.git")
                developerConnection.set("scm:git:ssh://git@github.com/4tkbytes/dropbear.git")
            }
        }
    }
}

tasks.register<Jar>("fatJar") {
    archiveClassifier.set("all")
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE

    from(kotlin.jvm().compilations["main"].output)

    configurations.named("jvmRuntimeClasspath").get().forEach { file ->
        if (file.name.endsWith(".jar")) {
            from(zipTree(file))
        } else {
            from(file)
        }
    }

    manifest {}
}