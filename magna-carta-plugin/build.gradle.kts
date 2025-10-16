import org.gradle.kotlin.dsl.compileOnly

plugins {
    `kotlin-dsl`
    `maven-publish`
    id("com.gradle.plugin-publish") version "2.0.0"
}

group = "com.dropbear"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation("de.undercouch:gradle-download-task:5.6.0")
    compileOnly("org.jetbrains.kotlin:kotlin-gradle-plugin:${KotlinVersion.CURRENT}")
}

gradlePlugin {
    website.set("https://github.com/4tkbytes/dropbear")
    vcsUrl.set("https://github.com/4tkbytes/dropbear")
    plugins {

        create("magnaCartaPlugin") {
            id = "magna-carta"
            implementationClass = "com.dropbear.magna_carta.MagnaCartaPlugin"
            displayName = "magna-carta plugin"
            description = "Gradle plugin for generating manifests from annotation data during compile time" +
                    " for use with KMP and the dropbear engine"
            version = version as String
        }
    }
}