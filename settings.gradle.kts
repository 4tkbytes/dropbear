rootProject.name = "dropbear"

pluginManagement {
    repositories {
        mavenCentral()
        gradlePluginPortal()
    }
}
plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "0.8.0"
}
include("magna-carta-plugin")

buildCache {
    local {
        isEnabled = true
    }
}
include("dropbear-gradle-plugin")