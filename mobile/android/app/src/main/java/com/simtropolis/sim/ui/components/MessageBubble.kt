package com.simtropolis.sim.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.sim.data.model.Message
import com.simtropolis.sim.data.model.MessageContent
import com.simtropolis.sim.data.model.MessageRole
import com.simtropolis.sim.data.model.ToolCall
import kotlinx.serialization.json.JsonElement
import com.simtropolis.sim.ui.theme.SimColors

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun MessageBubble(
    message: Message,
    onLongPress: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    val isUser = message.role == MessageRole.USER
    val bubbleColor = if (isUser) {
        SimColors.userBubble()
    } else {
        SimColors.assistantBubble()
    }
    val textColor = if (isUser) {
        SimColors.userBubbleText()
    } else {
        SimColors.assistantBubbleText()
    }

    val bubbleShape = RoundedCornerShape(
        topStart = 20.dp,
        topEnd = 20.dp,
        bottomStart = if (isUser) 20.dp else 6.dp,
        bottomEnd = if (isUser) 6.dp else 20.dp
    )

    Column(
        modifier = modifier.fillMaxWidth(),
        horizontalAlignment = if (isUser) Alignment.End else Alignment.Start
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = 320.dp)
                .clip(bubbleShape)
                .background(bubbleColor)
                .combinedClickable(
                    onClick = {},
                    onLongClick = { onLongPress?.invoke() }
                )
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            Column {
                message.content.forEach { content ->
                    when (content) {
                        is MessageContent.Text -> {
                            if (content.text.isNotBlank()) {
                                MarkdownText(
                                    text = content.text,
                                    textColor = textColor,
                                    modifier = Modifier.fillMaxWidth()
                                )
                            }
                        }

                        is MessageContent.ToolRequest -> {
                            ToolRequestView(
                                toolCall = content.toolCall,
                                textColor = textColor
                            )
                        }

                        is MessageContent.ToolResponse -> {
                            ToolResponseView(
                                toolResult = content.toolResult,
                                textColor = textColor
                            )
                        }

                        is MessageContent.Thinking -> {
                            ThinkingView(
                                thinking = content.thinking,
                                textColor = textColor
                            )
                        }

                        is MessageContent.ToolConfirmationRequest -> {
                            ToolConfirmationView(
                                toolName = content.toolName,
                                arguments = content.arguments,
                                textColor = textColor
                            )
                        }

                        is MessageContent.ConversationCompacted -> {
                            Text(
                                text = "📝 ${content.msg}",
                                style = MaterialTheme.typography.bodySmall,
                                color = textColor.copy(alpha = 0.7f),
                                modifier = Modifier.padding(vertical = 4.dp)
                            )
                        }

                        is MessageContent.SystemNotification -> {
                            Text(
                                text = "ℹ️ ${content.msg}",
                                style = MaterialTheme.typography.bodySmall,
                                color = textColor.copy(alpha = 0.7f),
                                modifier = Modifier.padding(vertical = 4.dp)
                            )
                        }

                        is MessageContent.SummarizationRequested -> {
                            Text(
                                text = "📝 Conversation compacted",
                                style = MaterialTheme.typography.bodySmall,
                                color = textColor.copy(alpha = 0.5f),
                                modifier = Modifier.padding(vertical = 4.dp)
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ToolRequestView(
    toolCall: ToolCall,
    textColor: Color
) {
    val isDark = isSystemInDarkTheme()

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        shape = RoundedCornerShape(8.dp),
        color = SimColors.toolBackground()
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = "🔧",
                    fontSize = 14.sp
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = toolCall.name,
                    style = MaterialTheme.typography.labelMedium,
                    color = if (isDark) Color(0xFF64B5F6) else Color(0xFF1976D2),
                    fontFamily = FontFamily.Monospace
                )
            }
            if (toolCall.arguments.isNotEmpty()) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = toolCall.arguments.toString(),
                    style = MaterialTheme.typography.bodySmall,
                    color = textColor.copy(alpha = 0.7f),
                    fontFamily = FontFamily.Monospace,
                    fontSize = 11.sp,
                    maxLines = 3
                )
            }
        }
    }
}

@Suppress("UNUSED_PARAMETER")
@Composable
private fun ToolResponseView(
    toolResult: com.simtropolis.sim.data.model.ToolResult,
    textColor: Color
) {
    val statusColor = when (toolResult.status) {
        "success" -> SimColors.Success
        "error" -> SimColors.Error
        else -> SimColors.Info
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        shape = RoundedCornerShape(8.dp),
        color = SimColors.toolBackground()
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = if (toolResult.status == "success") "✓" else "✗",
                    fontSize = 14.sp,
                    color = statusColor
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = toolResult.status.replaceFirstChar { it.uppercase() },
                    style = MaterialTheme.typography.labelMedium,
                    color = statusColor
                )
            }
            toolResult.error?.let { error ->
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = error,
                    style = MaterialTheme.typography.bodySmall,
                    color = SimColors.Error.copy(alpha = 0.9f),
                    maxLines = 3
                )
            }
        }
    }
}

@Composable
private fun ThinkingView(
    thinking: String,
    textColor: Color
) {
    var isExpanded by remember { mutableStateOf(false) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        shape = RoundedCornerShape(8.dp),
        color = SimColors.toolBackground()
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { isExpanded = !isExpanded }
            ) {
                Text(
                    text = if (isExpanded) "▼" else "▶",
                    fontSize = 10.sp,
                    color = textColor.copy(alpha = 0.5f)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "💭",
                    fontSize = 14.sp
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "Thinking Process",
                    style = MaterialTheme.typography.labelMedium,
                    color = textColor.copy(alpha = 0.7f)
                )
            }
            AnimatedVisibility(
                visible = isExpanded,
                enter = expandVertically(),
                exit = shrinkVertically()
            ) {
                if (thinking.isNotBlank()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = thinking,
                        style = MaterialTheme.typography.bodySmall,
                        color = textColor.copy(alpha = 0.6f)
                    )
                }
            }
        }
    }
}

@Composable
private fun ToolConfirmationView(
    toolName: String,
    arguments: Map<String, JsonElement>,
    textColor: Color
) {
    var isExpanded by remember { mutableStateOf(true) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        shape = RoundedCornerShape(8.dp),
        color = Color.Blue.copy(alpha = 0.08f)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { isExpanded = !isExpanded }
            ) {
                Text(
                    text = if (isExpanded) "▼" else "▶",
                    fontSize = 10.sp,
                    color = textColor.copy(alpha = 0.5f)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "❓",
                    fontSize = 14.sp
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "Permission: $toolName",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary
                )
                if (!isExpanded) {
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "(action required)",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }

            AnimatedVisibility(
                visible = isExpanded,
                enter = expandVertically(),
                exit = shrinkVertically()
            ) {
                Column {
                    Divider(modifier = Modifier.padding(vertical = 4.dp))

                    if (arguments.isNotEmpty()) {
                        Text(
                            text = "Arguments:",
                            style = MaterialTheme.typography.labelSmall,
                            color = textColor.copy(alpha = 0.6f)
                        )
                        arguments.forEach { (key, value) ->
                            Row(modifier = Modifier.padding(start = 8.dp, top = 2.dp)) {
                                Text(
                                    text = "$key: ",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = textColor.copy(alpha = 0.7f),
                                    fontFamily = FontFamily.Monospace
                                )
                                Text(
                                    text = value.toString(),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = textColor,
                                    fontFamily = FontFamily.Monospace
                                )
                            }
                        }
                    }

                    // Permission action buttons (stub - matching iOS TODO)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 8.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly
                    ) {
                        OutlinedButton(onClick = { /* TODO: Deny permission */ }) {
                            Text("Deny", color = Color.Red)
                        }
                        OutlinedButton(onClick = { /* TODO: Allow Once */ }) {
                            Text("Allow Once")
                        }
                        Button(onClick = { /* TODO: Always Allow */ }) {
                            Text("Always Allow")
                        }
                    }
                }
            }
        }
    }
}
