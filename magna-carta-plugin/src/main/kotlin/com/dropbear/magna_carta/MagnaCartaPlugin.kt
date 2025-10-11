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
        val extension = project.extensions.create("magna-carta", MagnaCartaExtension::class)

        val downloadToolTask = project.tasks.register("downloadMagnaCartaTool", DownloadMagnaCartaToolTask::class) {
            toolVersion.set("magna-carta-v0.0.1")
            outputDir.set(project.gradle.gradleUserHomeDir.resolve("magna-carta"))
        }

        val generateJvmTask = project.tasks.register("generateMagnaCartaJvm", GenerateMagnaCartaTask::class) {
            dependsOn(downloadToolTask)
            toolExecutable.set(downloadToolTask.flatMap { it.outputFile })
            target.set("jvm")
            inputDir.set(project.projectDir.resolve("src"))
            outputDir.set(project.layout.buildDirectory.dir("generated/magna-carta/jvm"))
        }

        val generateNativeTask = project.tasks.register("generateMagnaCartaNative", GenerateMagnaCartaTask::class) {
            dependsOn(downloadToolTask)
            toolExecutable.set(downloadToolTask.flatMap { it.outputFile })
            target.set("native")
            inputDir.set(project.projectDir.resolve("src"))
            outputDir.set(project.layout.buildDirectory.dir("generated/magna-carta/native"))
        }

        project.pluginManager.withPlugin("org.jetbrains.kotlin.multiplatform") {
            val kotlin = project.extensions.getByType(KotlinMultiplatformExtension::class)

            kotlin.sourceSets.apply {
                if (names.contains("jvmMain")) {
                    val jvmMain = getByName("jvmMain")
                    jvmMain.kotlin.srcDir(generateJvmTask.map { it.outputDir })
                }

                val nativeMain = findByName("nativeMain") ?: create("nativeMain")
                nativeMain.kotlin.srcDir(generateNativeTask.map { it.outputDir })

                kotlin.targets.withType(KotlinNativeTarget::class.java) {
                    compilations.getByName("main").defaultSourceSet.dependsOn(nativeMain)
                }
            }


            kotlin.targets.all {
                this.compilations.all {
                    this.compileTaskProvider.configure {
                        this.dependsOn(generateJvmTask, generateNativeTask)
                    }
                }
            }
        }
    }
}

abstract class MagnaCartaExtension { }