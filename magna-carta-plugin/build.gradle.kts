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

publishing {
    publications {
        withType<MavenPublication>().configureEach {
            pom {
                name.set("magna-carta")
                description.set("Gradle plugin for manifest generation for the Dropbear engine")
                url.set("https://4tkbytes.github.io/dropbear/")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }
                developers {
                    developer {
                        id.set("4tkbytes")
                        name.set("4tkbytes")
                        email.set("4tkbytes@pm.me")
                    }
                }
                scm {
                    connection.set("scm:git:git://github.com/4tkbytes/dropbear.git")
                    developerConnection.set("scm:git:ssh://github.com/4tkbytes/dropbear.git")
                    url.set("https://github.com/4tkbytes/dropbear")
                }
            }
        }
    }

    repositories {
        maven {
            name = "GitHubPages"
            url = uri("${layout.buildDirectory}/repo")
        }
    }
}