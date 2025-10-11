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
                "4cbfeca26ff0cfca5342dbd748066978041c213e6e3eb05214eec0f18a5b37b7"
            )
            os.isMacOsX && arch == "aarch64" -> Triple(
                "magna-carta-macos-arm64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-arm64",
                "446a89a9077fc78674342e3a16308957e9b39c347d3a313de00056630a446bb4"
            )
            os.isMacOsX && (arch == "x86_64" || arch == "amd64") -> Triple(
                "magna-carta-macos-x64",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-macos-x64",
                "5705f0ef43aab59e9f6b1b35fec391cceb373213f275b12d85e3614272ec0660"
            )
            os.isWindows && arch == "aarch64" -> Triple(
                "magna-carta-windows-arm64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-arm64.exe",
                "39072fab25a69ff8c321638897b7df00d5782c6a46b578eafbaedcb9d4efc53b"
            )
            os.isWindows && (arch == "x86_64" || arch == "amd64") -> Triple(
                "magna-carta-windows-x64.exe",
                "https://github.com/4tkbytes/dropbear/releases/download/${toolVersion.get()}/magna-carta-windows-x64.exe",
                "cce4c645da4e8a929a0f77f6685dcb104f955c19a7caba55e11d9b4c1c0ad077"
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