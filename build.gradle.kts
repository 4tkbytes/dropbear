plugins {
    kotlin("jvm") version "2.1.21"
}

group = "com.dropbear"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))
    implementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}
kotlin {
    jvmToolchain(21)
}

sourceSets {
    main {
        java.srcDirs("src/main/kotlin", "src/main/java")
    }
}

tasks.register<JavaCompile>("generateJniHeaders") {
    val outputDir = layout.buildDirectory.dir("generated/jni-headers")
    options.headerOutputDirectory.set(outputDir.get().asFile)

    classpath = files(
        tasks.named("compileKotlin"),
        tasks.named("compileJava")
    )

    source = fileTree("src/main/java") {
        include("**/*.java")
    }

    dependsOn("compileJava", "compileKotlin")
}
