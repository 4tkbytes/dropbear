package com.dropbear

@Target(AnnotationTarget.CLASS)
@Retention(AnnotationRetention.SOURCE)
annotation class Runnable(val tags: Array<String> = [])
