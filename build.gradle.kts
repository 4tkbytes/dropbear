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

buildscript {
    repositories {
        mavenCentral()
    }
    dependencies {
        classpath("io.github.cdimascio:dotenv-kotlin:6.4.1")
    }
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

    if (file("${project.rootDir}/target/debug/$libName").exists()) {
        println("Debug library exists")
//        "${project.rootDir}/target/debug/$libName"
    } else if (file("${project.rootDir}/target/release/$libName").exists()) {
        println("Release library exists")
//        "${project.rootDir}/target/debug/$libName"
    } else if (file("${project.rootDir}/libs/$libName").exists()) {
        println("Local library exists")
//        "${project.rootDir}/libs/$libName"
    } else {
        throw GradleException("libeucalyptus_core.so does not exist. This is a local build, so most likely you haven't built the rust library yet. \n" +
                "Try running cargo build")
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
                api("co.touchlab:kermit:2.0.4")
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

val dotenv = io.github.cdimascio.dotenv.dotenv()

publishing {
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/4tkbytes/dropbear")
            credentials {
                username = dotenv["GITHUB_USERNAME"]
                password = dotenv["GITHUB_TOKEN"]
            }
        }
    }

    publications {
        create<MavenPublication>("release") {
            groupId = group as String?
            artifactId = rootProject.name
            version = version

            from(components["kotlin"])
        }
    }
}