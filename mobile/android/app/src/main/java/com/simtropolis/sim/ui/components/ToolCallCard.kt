package com.simtropolis.sim.ui.components

import androidx.compose.animation.*
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.sim.data.model.MessageContent
import com.simtropolis.sim.data.model.ToolCall
import com.simtropolis.sim.data.model.ToolResult

/** A single tool call state — either loading or completed. */
sealed class ToolCallState {
    abstract val id: String
    abstract val toolCall: ToolCall

    data class Active(
        override val id: String,
        override val toolCall: ToolCall,
        val startTime: Long = System.currentTimeMillis()
    ) : ToolCallState()

    data class Completed(
        override val id: String,
        override val toolCall: ToolCall,
        val result: ToolResult,
        val durationMs: Long,
        val completedAt: Long = System.currentTimeMillis()
    ) : ToolCallState()
}

/** Timing wrapper, matching iOS ToolCallWithTiming. */
data class ToolCallWithTiming(
    val id: String = "",
    val toolCall: ToolCall,
    val startTime: Long = System.currentTimeMillis()
)

/** Completed tool call data, matching iOS CompletedToolCall. */
data class CompletedToolCallData(
    val toolCall: ToolCall,
    val result: ToolResult,
    val durationMs: Long,
    val completedAt: Long = System.currentTimeMillis()
)

@Composable
fun ToolCallCard(
    toolCallState: ToolCallState,
    onClick: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    val name: String
    val statusIcon: androidx.compose.ui.graphics.vector.ImageVector
    val statusColor: Color
    val duration: String

    when (toolCallState) {
        is ToolCallState.Active -> {
            val elapsed = (System.currentTimeMillis() - toolCallState.startTime) / 1000
            name = toolCallState.toolCall.name
            statusIcon = Icons.Default.HourglassEmpty
            statusColor = MaterialTheme.colorScheme.primary
            duration = "$elapsed s"
        }

        is ToolCallState.Completed -> {
            val success = toolCallState.result.status == "success"
            name = toolCallState.toolCall.name
            statusIcon = if (success) Icons.Default.CheckCircle else Icons.Default.Error
            statusColor = if (success) Color(0xFF4CAF50) else Color(0xFFF44336)
            duration = formatDuration(toolCallState.durationMs)
        }
    }

    Surface(
        modifier = modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier),
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
        tonalElevation = 1.dp
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Status icon
            Icon(
                imageVector = statusIcon,
                contentDescription = null,
                modifier = Modifier.size(20.dp),
                tint = statusColor
            )

            Spacer(modifier = Modifier.width(12.dp))

            // Tool name
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = name,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    fontFamily = FontFamily.Monospace
                )
            }

            // Duration
            Text(
                text = duration,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

private fun formatDuration(ms: Long): String {
    return when {
        ms < 1000 -> "${ms}ms"
        ms < 60_000 -> "${ms / 1000}.${(ms % 1000) / 100}s"
        else -> "${ms / 60_000}m ${(ms % 60_000) / 1000}s"
    }
}
