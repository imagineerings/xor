package com.simtropolis.baymax.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.simtropolis.baymax.data.model.Message
import com.simtropolis.baymax.data.model.MessageContent

/**
 * Full-screen overlay for reading long message content.
 * Mirrors iOS FullTextOverlay.
 */
@Composable
fun FullTextOverlay(
    message: Message,
    onDismiss: () -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text("Done")
            }
        },
        title = {
            Text(
                text = message.role.name.lowercase().replaceFirstChar { it.uppercase() },
                style = MaterialTheme.typography.titleMedium
            )
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
            ) {
                message.content.forEach { content ->
                    when (content) {
                        is MessageContent.Text -> {
                            if (content.text.isNotBlank()) {
                                Text(
                                    text = content.text,
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                                Spacer(modifier = Modifier.height(8.dp))
                            }
                        }

                        is MessageContent.ToolRequest -> {
                            Text(
                                text = "🔧 ${content.toolCall.name}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                        }

                        is MessageContent.ToolResponse -> {
                            Text(
                                text = "${if (content.toolResult.status == "success") "✓" else "✗"} Tool Response",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                        }

                        is MessageContent.Thinking -> {
                            Text(
                                text = "💭 ${content.thinking}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                        }

                        else -> {}
                    }
                }
            }
        }
    )
}
