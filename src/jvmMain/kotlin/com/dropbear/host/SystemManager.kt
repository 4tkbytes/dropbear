package com.dropbear.host

import com.dropbear.logging.Logger
import com.dropbear.logging.LogLevel
import com.dropbear.logging.LogWriter
import com.dropbear.logging.StdoutWriter

@Suppress("UNUSED")
class SystemManager(
    jarPath: String,
    private val engine: Any,
    logWriter: LogWriter? = null,
    logLevel: LogLevel? = null,
    logTarget: String = "unset"
) {
    private val hotSwapUtility = HotSwapUtility(jarPath, "com.dropbear.decl.RunnableRegistry")
    private var registryInstance: Any? = null
    private var registryClass: Class<*>? = null
    private val activeSystems = mutableMapOf<String, MutableList<Any>>()

    init {
        val writerToUse = logWriter ?: StdoutWriter()
        Logger.init(writerToUse, logLevel ?: LogLevel.INFO, logTarget)
        Logger.info("SystemManager: Initialised with jarPath: $jarPath, " +
                "logWriter: $writerToUse, " +
                "logLevel: $logLevel, " +
                "logTarget: $logTarget")

        val (instance, clazz) = loadRegistry()
        registryInstance = instance
        registryClass = clazz
    }

    private fun loadRegistry(): Pair<Any, Class<*>> {
        Logger.debug("Loading RunnableRegistry instance...")
        val instance = hotSwapUtility.getInstance(emptyArray(), emptyArray())
        Logger.debug("RunnableRegistry instance loaded successfully.")
        return instance to instance.javaClass
    }

    fun loadSystemsForTag(tag: String) {
        Logger.debug("Loading systems for tag: $tag")
        val instantiateMethod = registryClass?.getMethod("instantiateScripts", String::class.java)
        val systems = instantiateMethod?.invoke(registryInstance, tag) as List<*>

        val loadedSystems = mutableListOf<Any>()
        val engineClass = engine.javaClass

        for (system in systems) {
            system?.let {
                val loadMethod = it.javaClass.getMethod("load", engineClass)
                loadMethod.invoke(it, engine)
                loadedSystems.add(it)
                Logger.trace("Loaded system: ${it.javaClass.name} for tag: $tag")
            }
        }

        activeSystems[tag] = loadedSystems
        Logger.debug("Loaded ${loadedSystems.size} systems for tag: $tag")
    }

    fun updateAllSystems(deltaTime: Float) {
        Logger.trace("Updating all systems")
        val engineClass = engine.javaClass

        for ((_, systems) in activeSystems) {
            for (system in systems) {
                val updateMethod = system.javaClass.getMethod(
                    "update",
                    engineClass,
                    Float::class.javaPrimitiveType
                )
                updateMethod.invoke(system, engine, deltaTime)
            }
        }
    }

    fun updateSystemsByTag(tag: String, deltaTime: Float) {
        Logger.trace("Updating systems for tag: $tag")
        val systems = activeSystems[tag] ?: return
        val engineClass = engine.javaClass

        for (system in systems) {
            val updateMethod = system.javaClass.getMethod(
                "update",
                engineClass,
                Float::class.javaPrimitiveType
            )
            updateMethod.invoke(system, engine, deltaTime)
        }
    }

    fun reloadJar(newJarPath: String) {
        Logger.info("Reloading systems with new jar path: $newJarPath")
        activeSystems.clear()
        hotSwapUtility.reloadJar(newJarPath)

        val (instance, clazz) = loadRegistry()
        registryInstance = instance
        registryClass = clazz

        val reloadMethod = registryClass?.getMethod("reload")
        reloadMethod?.invoke(registryInstance)
        Logger.info("JAR loaded successfully.")
    }

    fun unloadSystemsByTag(tag: String) {
        activeSystems.remove(tag)
    }

    fun unloadAllSystems() {
        activeSystems.clear()
    }

    fun getSystemCount(tag: String): Int {
        return activeSystems[tag]?.size ?: 0
    }

    fun getTotalSystemCount(): Int {
        return activeSystems.values.sumOf { it.size }
    }

    fun getActiveTags(): Set<String> {
        return activeSystems.keys.toSet()
    }

    fun hasSystemsForTag(tag: String): Boolean {
        return activeSystems.containsKey(tag) && activeSystems[tag]?.isNotEmpty() == true
    }
}