plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.kotlinxSerialization)
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
            }
        }
    }

    sourceSets {
        commonMain {
            dependencies {
                implementation("co.touchlab:kermit:2.0.4")
            }
        }
        nativeMain {
            dependencies {
                implementation(libs.kotlinxSerializationJson)
            }
        }

        jvmMain {
            kotlin.srcDirs("src/jvmMain/kotlin", "src/jvmMain/java")
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