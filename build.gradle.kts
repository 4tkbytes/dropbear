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

kotlin {
    jvm()

    val hostOs = System.getProperty("os.name")
    val isArm64 = System.getProperty("os.arch") == "aarch64"
    val isMingwX64 = hostOs.startsWith("Windows")
    val nativeTarget = when {
        hostOs == "Mac OS X" && isArm64 -> macosArm64("nativeLib")
        hostOs == "Mac OS X" && !isArm64 -> macosX64("nativeLib")
        hostOs == "Linux" && isArm64 -> linuxArm64("nativeLib")
        hostOs == "Linux" && !isArm64 -> linuxX64("nativeLib")
        isMingwX64 -> mingwX64("nativeLib")
        else -> throw GradleException("Host OS is not supported in Kotlin/Native.")
    }

    val libName = when {
        hostOs == "Mac OS X" -> "libeucalyptus_core.dylib"
        hostOs == "Linux" -> "libeucalyptus_core.so"
        isMingwX64 -> "eucalyptus_core.dll"
        else -> throw GradleException("Host OS is not supported in Kotlin/Native.")
    }

    val (libDir, libNameForLinking) = when {
        file("${project.rootDir}/target/debug").exists() -> {
            val debugLibDir = "${project.rootDir}/target/debug"
            if (isMingwX64) {
                Pair(debugLibDir, "eucalyptus_core")
            } else {
                Pair(debugLibDir, "eucalyptus_core")
            }
        }
        file("${project.rootDir}/target/release").exists() -> {
            val releaseLibDir = "${project.rootDir}/target/release"
            if (isMingwX64) {
                Pair(releaseLibDir, "eucalyptus_core")
            } else {
                Pair(releaseLibDir, "eucalyptus_core")
            }
        }
        file("${project.rootDir}/libs").exists() -> {
            val libsDir = "${project.rootDir}/libs"
            if (isMingwX64) {
                Pair(libsDir, "eucalyptus_core")
            } else {
                Pair(libsDir, "eucalyptus_core")
            }
        }
        else -> {
            println("WARNING: Rust library directory not found!")
            Pair(null, null)
        }
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

                if (libDir != null && libNameForLinking != null) {
                    if (isMingwX64) {
                        linkerOpts("${libDir}/${libName}.lib")
                    } else {
                        linkerOpts("-L${libDir}", "-l${libNameForLinking}")
                    }
                }
            }
        }
    }

    sourceSets {
        commonMain {
            dependencies {
                api("co.touchlab:kermit:2.0.4")
            }
        }
        nativeMain {
            dependencies {
                implementation(libs.kotlinxSerializationJson)
            }
        }

        jvmMain {
            kotlin.srcDirs("src/jvmMain/kotlin", "src/jvmMain/java", "build/magna-carta")
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