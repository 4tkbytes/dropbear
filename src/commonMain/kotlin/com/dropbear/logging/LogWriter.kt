package com.dropbear.logging

interface LogWriter {
    fun log(level: LogLevel, target: String, message: String, file: String?, line: Int?)
}