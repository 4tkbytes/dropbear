package com.dropbear.host

import com.dropbear.DropbearEngine
import com.dropbear.System
import com.dropbear.logging.LogLevel
import com.dropbear.logging.LogWriter
import com.dropbear.logging.Logger
import com.dropbear.logging.StdoutWriter
import kotlin.collections.emptyList

@Suppress("UNUSED")
class SystemManager(
    jarPath: String,
    private val engine: DropbearEngine,
    logWriter: LogWriter? = null,
    logLevel: LogLevel? = null,
    logTarget: String = "unset"
) {
    private val hotSwapUtility = HotSwapUtility(jarPath, "com.dropbear.decl.RunnableRegistry")
    private var registryInstance: Any? = null
    private var registryClass: Class<*>? = null
    private val activeSystems = mutableMapOf<String, MutableList<System>>()

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
        val systems = instantiateMethod?.invoke(registryInstance, tag) as? List<*>

        val loadedSystems = mutableListOf<System>()

        if (systems != null) {
            for (system in systems) {
                val typed = system as? System
                if (typed == null) {
                    Logger.warn("Skipping script instance that does not extend com.dropbear.System: ${system?.javaClass?.name}")
                    continue
                }

                try {
                    typed.attachEngine(engine)
                    typed.clearCurrentEntity()
                    typed.load(engine)
                    loadedSystems.add(typed)
                    Logger.trace("Loaded system: ${typed.javaClass.name} for tag: $tag")
                } catch (ex: Exception) {
                    Logger.error("Failed to load system ${typed.javaClass.name}: ${ex.message}")
                }
            }
        } else {
            Logger.warn("No systems found for tag: $tag")
        }

        activeSystems[tag] = loadedSystems
        Logger.debug("Loaded ${loadedSystems.size} systems for tag: $tag")
    }

    fun updateAllSystems(deltaTime: Float) {
        Logger.trace("Updating all systems")
        for ((tag, systems) in activeSystems) {
            updateSystemsInternal(tag, systems, deltaTime)
        }
    }

    fun updateSystemsByTag(tag: String, deltaTime: Float) {
        Logger.trace("Updating systems for tag: $tag")
        val systems = activeSystems[tag] ?: return
        updateSystemsInternal(tag, systems, deltaTime)
    }

    fun updateSystemsForEntities(tag: String, entityIds: LongArray, deltaTime: Float) {
        Logger.trace("Updating systems for tag: $tag with ${entityIds.size} entities")
        val systems = activeSystems[tag] ?: return

        if (systems.isEmpty()) {
            return
        }

        if (entityIds.isEmpty()) {
            updateSystemsInternal(tag, systems, deltaTime)
            return
        }

        for (entityId in entityIds) {
            for (system in systems) {
                try {
                    system.attachEngine(engine)
                    system.setCurrentEntity(entityId)
                    system.update(engine, deltaTime)
                } catch (ex: Exception) {
                    Logger.error("Failed to update system ${system.javaClass.name} for entity $entityId: ${ex.message}")
                }
            }
        }

        for (system in systems) {
            system.clearCurrentEntity()
        }
    }

    private fun updateSystemsInternal(tag: String, systems: List<System>, deltaTime: Float) {
        for (system in systems) {
            try {
                system.attachEngine(engine)
                system.clearCurrentEntity()
                system.update(engine, deltaTime)
            } catch (ex: Exception) {
                Logger.error("Failed to update system ${system.javaClass.name} for tag $tag: ${ex.message}")
            }
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

    fun getSystemCount(tag: String): Int = activeSystems[tag]?.size ?: 0

    fun getTotalSystemCount(): Int = activeSystems.values.sumOf { it.size }

    fun getActiveTags(): Set<String> = activeSystems.keys.toSet()

    fun hasSystemsForTag(tag: String): Boolean = activeSystems[tag]?.isNotEmpty() == true
}


