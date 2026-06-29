package com.simtropolis.baymax.data.repository

import android.content.Context
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf

object ThemeManager {
    private const val PREFS_NAME = "baymax_theme"
    private const val KEY_DARK_MODE = "dark_mode"

    private var appContext: Context? = null
    val isDarkMode: MutableState<Boolean> = mutableStateOf(false)

    fun initialize(context: Context) {
        appContext = context.applicationContext
        isDarkMode.value = preferences(context).getBoolean(KEY_DARK_MODE, false)
    }

    fun setDarkMode(enabled: Boolean) {
        isDarkMode.value = enabled
        appContext?.let { context ->
            preferences(context).edit().putBoolean(KEY_DARK_MODE, enabled).apply()
        }
    }

    fun toggle() {
        setDarkMode(!isDarkMode.value)
    }

    private fun preferences(context: Context) =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
}
