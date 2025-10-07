package com.dropbear

/**
 * The basic interface that all classes implement for the class to be run.
 */
sealed class RunnableScript {
    /**
     * A function that is run once during the lifetime of the entity.
     *
     * It can be used to set initial properties such as health and more.
     *
     * ALl classes that implement RunnableScript need to implement the load function.
     */
    abstract fun load()

    /**
     * A function that is run every frame during the lifetime of the entity.
     */
    abstract fun update()
}