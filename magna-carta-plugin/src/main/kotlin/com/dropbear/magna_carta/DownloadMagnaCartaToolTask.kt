package com.dropbear.magna_carta

import de.undercouch.gradle.tasks.download.Download
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFile
import org.gradle.api.provider.Property
import org.gradle.api.provider.Provider
import org.gradle.api.tasks.*
import org.gradle.internal.os.OperatingSystem

abstract class DownloadMagnaCartaToolTask: Download() {
    @get:Input
    abstract val toolVersion: Property<String>

    @get:Internal
    abstract val outputDir: DirectoryProperty

    @get:OutputFile
    val outputFile: Provider<RegularFile> = outputDir.file(getToolFileName())

    @TaskAction
    override fun download() {
        println("Just a heads up: You can build your own version of magna-carta from the dropbear repository, and then" +
                "symlink it to ~/.gradle/magna-carta under the name ${getToolFileName()}")

        val os = OperatingSystem.current()
        val arch = System.getProperty("os.arch")

        val (fileName, url) = when {
            os.isLinux && arch == "amd64" -> arrayOf(
                "magna-carta-linux-x64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-linux-x64",
            )
            os.isMacOsX && arch == "aarch64" -> arrayOf(
                "magna-carta-macos-arm64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-arm64",
            )
            os.isMacOsX && (arch == "x86_64" || arch == "amd64") -> arrayOf(
                "magna-carta-macos-x64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-x64",
            )
            os.isWindows && arch == "aarch64" -> arrayOf(
                "magna-carta-windows-arm64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-arm64.exe",
            )
            os.isWindows && (arch == "x86_64" || arch == "amd64") -> arrayOf(
                "magna-carta-windows-x64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-x64.exe",
            )
            else -> throw GradleException("Unsupported OS/arch: $os / $arch")
        }

        val outputFile = outputDir.get().file(fileName).asFile
        if (outputFile.exists()) {
            println("Using cached magna-carta tool: $outputFile")
            return
        }

        src(url)
        dest(outputFile)
        overwrite(false)
        onlyIfModified(true)

        super.download()

        if (!os.isWindows) {
            outputFile.setExecutable(true)
        }
    }

    private fun getToolFileName(): String {
        val os = OperatingSystem.current()
        val arch = System.getProperty("os.arch")
        return when {
            os.isLinux && arch == "amd64" -> "magna-carta-linux-x64"
            os.isMacOsX && arch == "aarch64" -> "magna-carta-macos-arm64"
            os.isMacOsX && (arch == "x86_64" || arch == "amd64") -> "magna-carta-macos-x64"
            os.isWindows && arch == "aarch64" -> "magna-carta-windows-arm64.exe"
            os.isWindows && (arch == "x86_64" || arch == "amd64") -> "magna-carta-windows-x64.exe"
            else -> throw GradleException("Unsupported OS/arch: $os / $arch")
        }
    }
}