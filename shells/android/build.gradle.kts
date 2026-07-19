plugins {
    // AGP 9.0+ includes Kotlin support built in — no separate
    // org.jetbrains.kotlin.android plugin needed (applying it alongside is
    // now a hard error, not just redundant).
    id("com.android.application") version "9.2.0" apply false
}
