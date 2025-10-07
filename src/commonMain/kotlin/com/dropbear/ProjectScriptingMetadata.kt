package com.dropbear

/**
 * A mandatory class defined in a eucalyptus project that "discovers" and attaches
 * tags to files for the registry to use.
 */
interface ProjectScriptingMetadata {
    /**
     * A function that is placed in Manifest.kt, and allows you to attach the required
     * tags to the correct files.
     *
     * The original method was to use Kotlin Symbol Processing, however Kotlin/Native
     * doesn't support KSP.
     */
    fun getScripts(): List<ScriptRegistration>
}