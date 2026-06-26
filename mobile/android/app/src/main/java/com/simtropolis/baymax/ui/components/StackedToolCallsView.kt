package com.simtropolis.baymax.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.HourglassEmpty
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.simtropolis.baymax.data.model.Message

/**
 * Displays stacked tool calls for consecutive tool-only assistant messages.
 * Mirrors iOS StackedToolCallsView.
 *
 * When collapsed, shows a count badge + first tool name.
 * When expanded, shows all tool call cards in sequence.
 */
@Composable
fun StackedToolCallsView(
    groupMessages: List<Message>,
    activeToolCalls: Map<String, ToolCallWithTiming>,
    completedToolCalls: Map<String, CompletedToolCallData>,
    modifier: Modifier = Modifier
) {
    var isExpanded by remember { mutableStateOf(false) }

    // Collect all tool call states from the group messages
    val toolCallStates = remember(groupMessages, activeToolCalls, completedToolCalls) {
        buildList {
            for (message in groupMessages) {
                for (content in message.content) {
                    when (content) {
                        is com.simtropolis.baymax.data.model.MessageContent.ToolRequest -> {
                            val completed = completedToolCalls[content.id]
                            if (completed != null) {
                                add(
                                    ToolCallState.Completed(
                                        id = content.id,
                                        toolCall = content.toolCall,
                                        result = completed.result,
                                        durationMs = completed.durationMs
                                    )
                                )
                            } else {
                                val active = activeToolCalls[content.id]
                                if (active != null) {
                                    add(
                                        ToolCallState.Active(
                                            id = content.id,
                                            toolCall = active.toolCall,
                                            startTime = active.startTime
                                        )
                                    )
                                } else {
                                    // Unknown — treat as active without timing
                                    add(
                                        ToolCallState.Active(
                                            id = content.id,
                                            toolCall = content.toolCall
                                        )
                                    )
                                }
                            }
                        }

                        else -> { /* skip non-tool content */
                        }
                    }
                }
            }
        }
    }

    if (toolCallStates.isEmpty()) return

    val count = toolCallStates.size
    val firstToolName = toolCallStates.firstOrNull()?.let {
        when (it) {
            is ToolCallState.Active -> it.toolCall.name
            is ToolCallState.Completed -> it.toolCall.name
        }
    } ?: "Tool"

    // Check if all are completed
    val allCompleted = toolCallStates.all { it is ToolCallState.Completed }
    val allSuccess = toolCallStates.all {
        it is ToolCallState.Completed && it.result.status == "success"
    }

    Surface(
        modifier = modifier
            .fillMaxWidth()
            .clickable { isExpanded = !isExpanded },
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
        tonalElevation = 1.dp
    ) {
        Column(
            modifier = Modifier.padding(12.dp)
        ) {
            // Header row — always visible
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Status indicator
                Text(
                    text = when {
                        allSuccess -> "✓"
                        allCompleted -> "⚠"
                        else -> "⟳"
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Bold,
                    color = when {
                        allSuccess -> androidx.compose.ui.graphics.Color(0xFF4CAF50)
                        allCompleted -> androidx.compose.ui.graphics.Color(0xFFFFC107)
                        else -> MaterialTheme.colorScheme.primary
                    }
                )

                Spacer(modifier = Modifier.width(8.dp))

                // Summary text
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "$count tool call${if (count != 1) "s" else ""}",
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Medium
                    )
                    Text(
                        text = firstToolName,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Expand/collapse icon
                Icon(
                    imageVector = if (isExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    contentDescription = if (isExpanded) "Collapse" else "Expand",
                    modifier = Modifier.size(20.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            // Expanded tool call cards
            AnimatedVisibility(
                visible = isExpanded,
                enter = expandVertically(),
                exit = shrinkVertically()
            ) {
                Column(
                    modifier = Modifier.padding(top = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    toolCallStates.forEach { state ->
                        ToolCallCard(
                            toolCallState = state,
                            modifier = Modifier.padding(start = 8.dp)
                        )
                    }
                }
            }
        }
    }
}
