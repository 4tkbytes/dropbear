package com.dropbear.logging

@Suppress("unused")
object Logger {
    private var writer: LogWriter = StdoutWriter()
    private var minLevel: LogLevel = LogLevel.INFO
    private var defaultTarget: String = "dropbear"

    internal fun init(writer: LogWriter, minLevel: LogLevel = LogLevel.INFO, defaultTarget: String = "dropbear") {
        this.writer = writer
        this.minLevel = minLevel
        this.defaultTarget = defaultTarget
        println("Log: Initialised with writer: $writer, minLevel: $minLevel, defaultTarget: $defaultTarget")
    }

    fun setLogLevel(level: LogLevel) {
        this.minLevel = level
    }

    private fun logInternal(level: LogLevel, message: String, target: String, file: String?, line: Int?) {
        if (level.ordinal >= minLevel.ordinal) {
            writer.log(level, target, message, file, line)
        }
    }

    fun trace(message: String, target: String = defaultTarget, file: String? = null, line: Int? = null) =
        logInternal(LogLevel.TRACE, message, target, file, line)
    fun debug(message: String, target: String = defaultTarget, file: String? = null, line: Int? = null) =
        logInternal(LogLevel.DEBUG, message, target, file, line)
    fun info(message: String, target: String = defaultTarget, file: String? = null, line: Int? = null) =
        logInternal(LogLevel.INFO, message, target, file, line)
    fun warn(message: String, target: String = defaultTarget, file: String? = null, line: Int? = null) =
        logInternal(LogLevel.WARN, message, target, file, line)
    fun error(message: String, target: String = defaultTarget, file: String? = null, line: Int? = null) =
        logInternal(LogLevel.ERROR, message, target, file, line)

    // ---

    fun trace(message: () -> String, target: String = defaultTarget, file: String? = null, line: Int? = null) {
        if (LogLevel.TRACE.ordinal >= minLevel.ordinal) {
            logInternal(LogLevel.TRACE, message(), target, file, line)
        }
    }
    fun debug(message: () -> String, target: String = defaultTarget, file: String? = null, line: Int? = null) {
        if (LogLevel.DEBUG.ordinal >= minLevel.ordinal) {
            logInternal(LogLevel.DEBUG, message(), target, file, line)
        }
    }
    fun info(message: () -> String, target: String = defaultTarget, file: String? = null, line: Int? = null) {
        if (LogLevel.INFO.ordinal >= minLevel.ordinal) {
            logInternal(LogLevel.INFO, message(), target, file, line)
        }
    }
    fun warn(message: () -> String, target: String = defaultTarget, file: String? = null, line: Int? = null) {
        if (LogLevel.WARN.ordinal >= minLevel.ordinal) {
            logInternal(LogLevel.WARN, message(), target, file, line)
        }
    }
    fun error(message: () -> String, target: String = defaultTarget, file: String? = null, line: Int? = null) {
        if (LogLevel.ERROR.ordinal >= minLevel.ordinal) {
            logInternal(LogLevel.ERROR, message(), target, file, line)
        }
    }
}