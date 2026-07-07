package com.simtropolis.sim.ui.screens

import android.content.Context
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.simtropolis.sim.R
import com.simtropolis.sim.ui.theme.SimColors
import kotlinx.coroutines.delay

@Composable
fun SplashScreen(isActive: MutableState<Boolean>) {
    val context = LocalContext.current
    val visible = remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        val preferences = context.getSharedPreferences("sim_splash", Context.MODE_PRIVATE)
        val now = System.currentTimeMillis()
        val hasLaunchedBefore = preferences.getBoolean("has_launched_before", false)
        val lastOpenTime = preferences.getLong("last_app_open_time", 0L)
        val longSplash = !hasLaunchedBefore || now - lastOpenTime > 86_400_000L
        val fadeInMs = if (longSplash) 400L else 200L
        val displayMs = if (longSplash) 1_000L else 300L
        val fadeOutMs = if (longSplash) 400L else 200L

        visible.value = true
        delay(fadeInMs + displayMs)
        visible.value = false
        delay(fadeOutMs)
        preferences.edit()
            .putBoolean("has_launched_before", true)
            .putLong("last_app_open_time", now)
            .apply()
        isActive.value = false
    }

    AnimatedVisibility(
        visible = visible.value,
        enter = fadeIn(),
        exit = fadeOut()
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(androidx.compose.material3.MaterialTheme.colorScheme.background),
            contentAlignment = Alignment.Center
        ) {
            Image(
                painter = painterResource(R.drawable.ic_sim_logo),
                contentDescription = "Sim",
                modifier = Modifier.size(120.dp),
                colorFilter = ColorFilter.tint(SimColors.logoTint())
            )
        }
    }
}
