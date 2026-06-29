package com.simtropolis.baymax.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import com.simtropolis.baymax.data.repository.ThemeManager

// Neutral/System colors - matching iOS which uses black/dark for primary
val BaymaxPrimary = Color(0xFF1C1C1E)  // Dark neutral - same as iOS dark mode background
val BaymaxPrimaryLight = Color(0xFF000000)  // Pure black for light mode accents

// Light Theme Colors - clean, minimal like iOS
private val LightColorScheme = lightColorScheme(
    primary = BaymaxPrimaryLight,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFF5F5F7),
    onPrimaryContainer = Color(0xFF1C1C1E),
    secondary = Color(0xFF2196F3),
    onSecondary = Color.White,
    secondaryContainer = Color(0xFFD6E8FF),
    onSecondaryContainer = Color(0xFF001B3D),
    tertiary = Color(0xFF4CAF50),
    onTertiary = Color.White,
    background = Color(0xFFFFFFFF),
    onBackground = Color(0xFF1C1C1E),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF1C1C1E),
    surfaceVariant = Color(0xFFF5F5F7),
    onSurfaceVariant = Color(0xFF666666),
    outline = Color(0xFFE0E0E0),
    outlineVariant = Color(0xFFEBEBED),
    error = Color(0xFFF44336),
    onError = Color.White
)

// Dark Theme Colors (matching iOS dark mode)
private val DarkColorScheme = darkColorScheme(
    primary = Color.White,  // White primary for dark mode
    onPrimary = Color(0xFF1C1C1E),
    primaryContainer = Color(0xFF2C2C2E),
    onPrimaryContainer = Color.White,
    secondary = Color(0xFF64B5F6),
    onSecondary = Color(0xFF003062),
    secondaryContainer = Color(0xFF004788),
    onSecondaryContainer = Color(0xFFD6E8FF),
    tertiary = Color(0xFF66BB6A),
    onTertiary = Color(0xFF003910),
    background = Color(0xFF1C1C1E),  // iOS dark mode background
    onBackground = Color.White,
    surface = Color(0xFF1C1C1E),
    onSurface = Color.White,
    surfaceVariant = Color(0xFF2C2C2E),
    onSurfaceVariant = Color(0xFFB0B0B0),
    outline = Color(0xFF48484A),
    outlineVariant = Color(0xFF38383A),
    error = Color(0xFFEF5350),
    onError = Color.White
)

// Additional Baymax-specific colors
object BaymaxColors {
    // Message bubbles - user bubbles are dark/black like iOS
    @Composable
    fun userBubble() = if (ThemeManager.isDarkMode.value) Color.White else Color(0xFF1C1C1E)
    
    @Composable
    fun userBubbleText() = if (ThemeManager.isDarkMode.value) Color(0xFF1C1C1E) else Color.White
    
    @Composable
    fun assistantBubble() = if (ThemeManager.isDarkMode.value) Color(0xFF2C2C2E) else Color(0xFFF0F0F2)
    
    @Composable
    fun assistantBubbleText() = if (ThemeManager.isDarkMode.value) Color.White else Color(0xFF1C1C1E)
    
    // Input field
    @Composable
    fun inputBackground() = if (ThemeManager.isDarkMode.value) Color(0xFF2C2C2E) else Color(0xFFF5F5F8)
    
    @Composable
    fun inputBorder() = if (ThemeManager.isDarkMode.value) Color(0xFF48484A) else Color(0xFFE0E0E0)
    
    // Tool/code backgrounds
    @Composable
    fun toolBackground() = if (ThemeManager.isDarkMode.value) Color(0xFF2C2C2E) else Color(0xFFF5F5F5)
    
    val CodeBackground = Color(0xFF1E1E1E)
    val CodeText = Color(0xFFD4D4D4)
    
    // Status colors
    val Success = Color(0xFF4CAF50)
    val Error = Color(0xFFF44336)
    val Warning = Color(0xFFFFC107)
    val Info = Color(0xFF2196F3)
    
    // Thinking animation
    val ThinkingCyan = Color(0xFF00BCD4)
    val ThinkingYellow = Color(0xFFFFEB3B)
    
    // Logo tint - dark in light mode, white in dark mode
    @Composable
    fun logoTint() = if (ThemeManager.isDarkMode.value) Color.White else Color(0xFF1C1C1E)
}

@Composable
fun BaymaxTheme(
    darkTheme: Boolean = ThemeManager.isDarkMode.value,
    dynamicColor: Boolean = false, // Disabled to use Baymax brand colors
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }
    
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = colorScheme.background.toArgb()
            WindowCompat.getInsetsController(window, view).isAppearanceLightStatusBars = !darkTheme
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
