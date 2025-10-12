package com.dropbear.magna_carta

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.create
import org.gradle.kotlin.dsl.getByType
import org.gradle.kotlin.dsl.register
import org.jetbrains.kotlin.gradle.dsl.KotlinMultiplatformExtension
import org.jetbrains.kotlin.gradle.plugin.mpp.KotlinNativeTarget

class MagnaCartaPlugin : Plugin<Project> {
    override fun apply(project: Project) {
        project.extensions.create("magna-carta", MagnaCartaExtension::class)

        val downloadToolTask = project.tasks.register("downloadMagnaCartaTool", DownloadMagnaCartaToolTask::class) {
            toolVersion.set("magna-carta-v0.0.1")
            outputDir.set(project.gradle.gradleUserHomeDir.resolve("magna-carta"))
        }

        val generateJvmTask = project.tasks.register("generateMagnaCartaJvm", GenerateMagnaCartaTask::class) {
            dependsOn(downloadToolTask)
            toolExecutable.set(downloadToolTask.flatMap { it.outputFile })
            target.set("jvm")
            inputDir.set(project.projectDir.resolve("src"))
            outputDir.set(project.layout.buildDirectory.dir("magna-carta/jvmMain"))
        }

        val generateNativeTask = project.tasks.register("generateMagnaCartaNative", GenerateMagnaCartaTask::class) {
            dependsOn(downloadToolTask)
            toolExecutable.set(downloadToolTask.flatMap { it.outputFile })
            target.set("native")
            inputDir.set(project.projectDir.resolve("src"))
            outputDir.set(project.layout.buildDirectory.dir("magna-carta/nativeLibMain"))
        }

        project.pluginManager.withPlugin("org.jetbrains.kotlin.multiplatform") {
            val kotlin = project.extensions.getByType(KotlinMultiplatformExtension::class)

            kotlin.sourceSets.apply {
                if (names.contains("jvmMain")) {
                    val jvmMain = getByName("jvmMain")
                    jvmMain.kotlin.srcDir(generateJvmTask.map { it.outputDir })
                }

                val nativeLibMain = findByName("nativeLibMain") ?: create("nativeLibMain")
                nativeLibMain.kotlin.srcDir(generateNativeTask.map { it.outputDir })

                kotlin.targets.withType(KotlinNativeTarget::class.java) {
                    compilations.getByName("main").defaultSourceSet.dependsOn(nativeLibMain)
                }
            }


            kotlin.targets.all {
                this.compilations.all {
                    this.compileTaskProvider.configure {
                        when (this.name) {
                            "compileKotlinNative" -> {
                                this.dependsOn(generateNativeTask)
                            }
                            "compileKotlinJvm" -> {
                                this.dependsOn(generateJvmTask)
                            }
                            else -> {
                                println("Unknown compilation task: ${this.name}")
                            }
                        }
                    }
                }
            }
        }
    }
}

abstract class MagnaCartaExtension { }