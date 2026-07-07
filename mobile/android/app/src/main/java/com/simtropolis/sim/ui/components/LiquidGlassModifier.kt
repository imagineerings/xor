package com.simtropolis.sim.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

fun Modifier.liquidGlassBackground(
    color: Color,
    cornerRadius: Dp = 32.dp
): Modifier {
    val shape = RoundedCornerShape(bottomStart = cornerRadius, bottomEnd = cornerRadius)
    val base = shadow(
        elevation = 16.dp,
        shape = shape,
        ambientColor = Color.Black.copy(alpha = 0.12f),
        spotColor = Color.Black.copy(alpha = 0.12f)
    )

    return base.background(color.copy(alpha = 0.94f), shape)
}
