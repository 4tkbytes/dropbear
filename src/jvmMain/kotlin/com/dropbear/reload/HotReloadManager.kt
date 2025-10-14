package com.dropbear.reload

import java.net.URLClassLoader
import java.nio.file.Paths

class HotReloadManager(private val jarPath: String) : AutoCloseable {
    @Volatile
    private var currentLoader: URLClassLoader? = null

    init {
        reload()
    }

    @Synchronized
    fun reload() {
        currentLoader?.close()

        val jarUrl = Paths.get(jarPath).toUri().toURL()

        currentLoader = URLClassLoader(
            arrayOf(jarUrl),
            ClassLoader.getSystemClassLoader().parent
        )
    }

    fun loadClass(className: String): Class<*> {
        return currentLoader?.loadClass(className)
            ?: throw IllegalStateException("ClassLoader not initialised")
    }

    fun createInstance(className: String): Any {
        val clazz = loadClass(className)
        return clazz.getDeclaredConstructor().newInstance()
    }

    fun getCurrentLoader(): ClassLoader {
        return currentLoader ?: throw IllegalStateException("ClassLoader not initialised")
    }

    override fun close() {
        currentLoader?.close()
        currentLoader = null
    }
}