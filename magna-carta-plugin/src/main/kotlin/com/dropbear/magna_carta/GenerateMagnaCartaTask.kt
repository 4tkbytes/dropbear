package com.dropbear.magna_carta

import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.*
import org.gradle.internal.os.OperatingSystem

abstract class GenerateMagnaCartaTask : DefaultTask() {

    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val toolExecutable: RegularFileProperty

    @get:Input
    abstract val target: Property<String>

    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val inputDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val tool = toolExecutable.get().asFile
        val input = inputDir.get().asFile
        val output = outputDir.get().asFile

        output.mkdirs()

        val os = OperatingSystem.current()
        val command = if (os.isWindows) {
            listOf(tool.absolutePath, "--input", input.absolutePath, "--output", output.absolutePath, "--target", target.get())
        } else {
            listOf("bash", "-c", "${tool.absolutePath} --input ${input.absolutePath} --output ${output.absolutePath} --target ${target.get()}")
        }

        project.exec {
            commandLine = command
            workingDir = project.projectDir
        }
    }
}