package com.atelier.bookscan

enum class CaptureMode {
    AUTO,
    SEMI,
    MANUAL;

    companion object {
        fun fromWire(value: String): CaptureMode = when (value.lowercase()) {
            "auto" -> AUTO
            "manual" -> MANUAL
            else -> SEMI
        }
    }

    fun toWire(): String = name.lowercase()
}

enum class PointState {
    SEARCHING,
    STABLE,
    ARMED;

    fun toWire(): String = name.lowercase()
}

data class AutoParams(
    var settleMs: Long = 600,
    var cooldownMs: Long = 1200,
    var motionThreshold: Float = 12f,
)
