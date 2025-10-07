package com.dropbear

internal object Registry {
    private val _entries = mutableListOf<ScriptEntry>()
    val entries: List<ScriptEntry> get() = _entries.toList()

    fun register(entry: ScriptEntry) {
        _entries.add(entry)
    }

    fun findByTag(tag: String): List<ScriptEntry> {
        return _entries.filter { tag in it.tags }
    }

    internal fun registerAll() {
    }
}