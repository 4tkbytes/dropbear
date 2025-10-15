package com.dropbear.logging

import kotlinx.datetime.Clock
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime

class StdoutWriter: LogWriter {
    override fun log(
        level: LogLevel,
        target: String,
        message: String,
        file: String?,
        line: Int?
    ) {
        val now = Clock.System.now()
        val timeZone = TimeZone.currentSystemDefault()
        val timestamp = now.toLocalDateTime(timeZone)
        val location = if (file != null && line != null) "[$file:$line] " else ""
        when (level) {
            LogLevel.ERROR, LogLevel.WARN -> error("[$timestamp] [$level] $location[$target] $message")
            else -> println("[$timestamp] [$level] $location[$target] $message")
        }
    }
}