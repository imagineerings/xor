package com.simtropolis.baymax.ui.components

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.simtropolis.baymax.data.model.ChatSession
import java.time.Instant
import java.time.ZoneId
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

@Composable
fun NodeMatrix(
    sessions: List<ChatSession>,
    selectedSessionId: String?,
    onNodeTap: (ChatSession) -> Unit,
    onDayChange: ((Int) -> Unit)?,
    favoriteIds: Set<String> = emptySet(),
    isLoading: Boolean = false,
    showDraftNode: Boolean = false,
    modifier: Modifier = Modifier
) {
    var daysOffset by remember { mutableStateOf(0) }
    var dragDistance by remember { mutableStateOf(0f) }
    val sessionsForDay = remember(sessions, daysOffset) {
        sessions.filter { isSessionOnDayOffset(it, daysOffset) }
    }
    val transition = rememberInfiniteTransition(label = "node-live")
    val pulse by transition.animateFloat(
        initialValue = 0.75f,
        targetValue = 1.2f,
        animationSpec = infiniteRepeatable(tween(900), RepeatMode.Reverse),
        label = "pulse"
    )

    BoxWithConstraints(
        modifier = modifier
            .height(220.dp)
            .pointerInput(sessionsForDay, daysOffset) {
                detectDragGestures(
                    onDragEnd = {
                        when {
                            dragDistance > 80f -> {
                                daysOffset += 1
                                onDayChange?.invoke(daysOffset)
                            }
                            dragDistance < -80f && daysOffset > 0 -> {
                                daysOffset -= 1
                                onDayChange?.invoke(daysOffset)
                            }
                        }
                        dragDistance = 0f
                    },
                    onDrag = { change, dragAmount ->
                        change.consume()
                        dragDistance += dragAmount.x
                    }
                )
            },
        contentAlignment = Alignment.Center
    ) {
        if (isLoading) {
            CircularProgressIndicator()
            return@BoxWithConstraints
        }

        if (sessionsForDay.isEmpty() && !showDraftNode) {
            Text(
                text = if (daysOffset == 0) "No sessions today" else "No sessions for this day",
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            return@BoxWithConstraints
        }

        val density = LocalDensity.current
        val widthPx = with(density) { maxWidth.toPx() }
        val heightPx = with(density) { maxHeight.toPx() }
        val nodePositions = remember(sessionsForDay, widthPx, heightPx) {
            computeNodePositions(sessionsForDay, widthPx, heightPx)
        }

        Canvas(
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(nodePositions) {
                    detectTapGestures { offset ->
                        nodePositions.firstOrNull { (_, center, radius) ->
                            (center - offset).getDistance() <= radius + 12f
                        }?.let { (session) -> onNodeTap(session) }
                    }
                }
        ) {
            for ((session, center, radius) in nodePositions) {
                val isSelected = session.id == selectedSessionId
                val isLive = isLiveSession(session)
                if (isLive) {
                    drawCircle(
                        color = Color(0xFF2196F3).copy(alpha = 0.18f),
                        radius = radius * pulse,
                        center = center
                    )
                }
                drawCircle(
                    color = if (isSelected) Color(0xFF2196F3) else Color(0xFF1C1C1E),
                    radius = radius,
                    center = center
                )
                drawCircle(
                    color = Color.White.copy(alpha = 0.28f),
                    radius = radius,
                    center = center,
                    style = Stroke(width = 2f)
                )
                if (session.id in favoriteIds) {
                    drawStar(center + Offset(radius * 0.65f, -radius * 0.65f), radius * 0.45f)
                }
            }
        }
    }
}

private data class NodePosition(
    val session: ChatSession,
    val center: Offset,
    val radius: Float
)

private fun computeNodePositions(
    sessions: List<ChatSession>,
    width: Float,
    height: Float
): List<NodePosition> {
    val columns = 4
    val horizontalGap = width / (columns + 1)
    val verticalGap = height / ((sessions.size / columns) + 2)
    return sessions.mapIndexed { index, session ->
        val column = index % columns
        val row = index / columns
        val radius = (8f + min(session.messageCount, 80) / 80f * 8f)
        NodePosition(
            session = session,
            center = Offset(horizontalGap * (column + 1), verticalGap * (row + 1)),
            radius = radius
        )
    }
}

private fun isSessionOnDayOffset(session: ChatSession, daysOffset: Int): Boolean {
    return try {
        val target = java.time.LocalDate.now().minusDays(daysOffset.toLong())
        Instant.parse(session.updatedAt).atZone(ZoneId.systemDefault()).toLocalDate() == target
    } catch (_: Exception) {
        daysOffset == 0
    }
}

private fun isLiveSession(session: ChatSession): Boolean {
    return try {
        val updatedAt = Instant.parse(session.updatedAt)
        Instant.now().minusSeconds(300).isBefore(updatedAt)
    } catch (_: Exception) {
        false
    }
}

private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawStar(center: Offset, radius: Float) {
    val path = Path()
    for (point in 0 until 10) {
        val angle = -PI / 2 + point * PI / 5
        val currentRadius = if (point % 2 == 0) radius else radius * 0.45f
        val offset = Offset(
            x = center.x + cos(angle).toFloat() * currentRadius,
            y = center.y + sin(angle).toFloat() * currentRadius
        )
        if (point == 0) path.moveTo(offset.x, offset.y) else path.lineTo(offset.x, offset.y)
    }
    path.close()
    drawPath(path, Color(0xFFFFD700))
}
