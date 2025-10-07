package com.dropbear

internal data class ScriptEntry(
    val tags: List<String>,
    val functionName: String,
    val invoker: (DropbearEngine, EntityId, Double) -> Unit
)