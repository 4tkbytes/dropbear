package com.dropbear.magna_carta

import de.undercouch.gradle.tasks.download.Download
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFile
import org.gradle.api.provider.Property
import org.gradle.api.provider.Provider
import org.gradle.api.tasks.*
import org.gradle.internal.os.OperatingSystem
import java.io.File
import java.security.MessageDigest

abstract class DownloadMagnaCartaToolTask: Download() {
    @get:Input
    abstract val toolVersion: Property<String>

    @get:Internal
    abstract val outputDir: DirectoryProperty

    @get:OutputFile
    val outputFile: Provider<RegularFile> = outputDir.file(getToolFileName())

    @TaskAction
    override fun download() {
        val os = OperatingSystem.current()
        val arch = System.getProperty("os.arch")

        val (fileName, url, expectedSha256) = when {
            os.isLinux && arch == "amd64" -> Triple(
                "magna-carta-linux-x64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-linux-x64",
                "88b3497ab7e787260aeb7f4d91fe46fa9f78ccfb32f841a27a693b824da4bc32"
            )
            os.isMacOsX && arch == "aarch64" -> Triple(
                "magna-carta-macos-arm64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-arm64",
                "73ba95d193ab7ac5925324e4c3006c60e88bf0cb89b7a08786b75f2bfc038c10"
            )
            os.isMacOsX && (arch == "x86_64" || arch == "amd64") -> Triple(
                "magna-carta-macos-x64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-x64",
                "012202ee74db0d9638033c8947b7f94d4b20be95b00f2ea181d892af20e226e2"
            )
            os.isWindows && arch == "aarch64" -> Triple(
                "magna-carta-windows-arm64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-arm64.exe",
                "639ab60a2693d82b548be1497ef25117a13b6e0c69fc45a19450fd10e2610d13"
            )
            os.isWindows && (arch == "x86_64" || arch == "amd64") -> Triple(
                "magna-carta-windows-x64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-x64.exe",
                "983880be951c9bda800a616739c4245f97e5ddb2818017d0d9c29ae62c0b9016"
            )
            else -> throw GradleException("Unsupported OS/arch: $os / $arch")
        }

        val outputFile = outputDir.get().file(fileName).asFile
        if (outputFile.exists() && verifyChecksum(outputFile, expectedSha256)) {
            println("Using cached magna-carta tool: $outputFile")
            return
        }

        src(url)
        dest(outputFile)
        overwrite(false)
        onlyIfModified(true)

        super.download()

        if (!verifyChecksum(outputFile, expectedSha256)) {
            outputFile.delete()
            throw GradleException("Checksum verification failed for $fileName")
        }

        if (!os.isWindows) {
            outputFile.setExecutable(true)
        }
    }

    private fun verifyChecksum(file: File, expectedSha256: String): Boolean {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(8192)
            var bytesRead: Int
            while (input.read(buffer).also { bytesRead = it } > 0) {
                digest.update(buffer, 0, bytesRead)
            }
        }
        val actualSha256 = digest.digest().joinToString("") { "%02x".format(it) }
        return actualSha256 == expectedSha256
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