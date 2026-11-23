plugins {
    id("com.android.application") version "9.2.1" apply false
    // AGP's built-in Kotlin compiles the sources; this puts the matching Kotlin Gradle plugin (and
    // its version) on the build's classpath, as the painting app does.
    id("org.jetbrains.kotlin.plugin.compose") version "2.3.21" apply false
}
