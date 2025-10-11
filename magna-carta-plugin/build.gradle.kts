import org.gradle.kotlin.dsl.compileOnly

plugins {
    `kotlin-dsl`
    `maven-publish`
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
    plugins {
        create("dropbearPlugin") {
            id = "magna-carta"
            implementationClass = "com.dropbear.magna_carta.MagnaCartaPlugin"
            version = "1.0-SNAPSHOT"
        }
    }
}