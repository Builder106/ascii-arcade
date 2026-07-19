package com.builder106.asciiarcade

import android.graphics.Color

// Named AaTheme, not Theme — a bare `Theme` in a file that also touches
// android.graphics/android.content.res APIs would shadow
// android.content.res.Resources.Theme.
data class AaTheme(val label: String, val text: Int, val background: Int)

/**
 * RGB values mirror crates/aa-core/src/theme.rs's Theme::ALL and
 * shells/ios/AsciiArcade/Themes.swift exactly. There is no aa_theme_names()
 * FFI export (iOS doesn't have one either), so these are hardcoded
 * client-side rather than fetched from the engine. Only HACKER is wired up
 * for this vertical slice — a picker over ALL is a fast-follow.
 */
object Themes {
    val HACKER = AaTheme("Hacker", Color.rgb(48, 209, 88), Color.BLACK)
    val AMBER = AaTheme("Amber", Color.rgb(255, 166, 0), Color.rgb(26, 8, 0))
    val ICE = AaTheme("Ice", Color.rgb(0, 255, 255), Color.rgb(0, 13, 26))
    val GHOST = AaTheme("Ghost", Color.rgb(28, 28, 30), Color.rgb(245, 245, 245))
    val ALL = listOf(HACKER, AMBER, ICE, GHOST)

    fun named(name: String): AaTheme = ALL.firstOrNull { it.label.equals(name, ignoreCase = true) } ?: HACKER
}
