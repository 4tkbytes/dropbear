package com.dropbear

/**
 * Determines a script that can be ran. 
 * 
 * This annotation will be searched for when run through
 * the `magna-carta` manifest generator tool. 
 * 
 * The tags correspond to the tags provided to the entity
 * with the Script. 
 */
@Target(AnnotationTarget.CLASS)
@Retention(AnnotationRetention.SOURCE)
annotation class Runnable(val tags: Array<String> = [])
