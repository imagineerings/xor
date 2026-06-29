package com.simtropolis.baymax.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.simtropolis.baymax.data.model.Message
import com.simtropolis.baymax.data.model.MessageContent
import com.simtropolis.baymax.ui.components.CompletedToolCallData
import java.text.SimpleDateFormat
import java.util.*

/**
 * Static holder for passing task detail data between screens.
 */
object TaskDetailData {
    var message: Message? = null
    var completedTasks: List<CompletedToolCallData> = emptyList()
    var sessionName: String? = null
}

/**
 * Static holder for passing task output detail data between screens.
 */
object TaskOutputDetailData {
    var task: CompletedToolCallData? = null
    var taskNumber: Int = 0
    var sessionName: String? = null
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskDetailScreen(
    onNavigateBack: () -> Unit,
    onNavigateToTaskOutput: () -> Unit
) {
    val message = TaskDetailData.message
    val completedTasks = TaskDetailData.completedTasks
    val sessionName = TaskDetailData.sessionName

    val taskName = when {
        completedTasks.size == 1 -> completedTasks[0].toolCall.name
        else -> "${completedTasks.size} Tasks"
    }

    val messageText = message?.content?.filterIsInstance<MessageContent.Text>()
        ?.firstOrNull()?.text.orEmpty()

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = sessionName.orEmpty(),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            text = taskName,
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.Default.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .verticalScroll(rememberScrollState())
                .padding(16.dp)
        ) {
            // Timestamp
            val timestamp = message?.created ?: 0L
            Text(
                text = formatTimestamp2(timestamp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Spacer(modifier = Modifier.height(16.dp))

            // Reasoning text
            if (messageText.isNotBlank()) {
                Text(
                    text = "Response",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = messageText,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Spacer(modifier = Modifier.height(16.dp))
            }

            // Completed tasks
            if (completedTasks.isNotEmpty()) {
                Text(
                    text = "Completed Tasks",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(8.dp))

                completedTasks.forEachIndexed { index, task ->
                    val success = task.result.status == "success"
                    Surface(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp)
                            .clickable {
                                TaskOutputDetailData.task = task
                                TaskOutputDetailData.taskNumber = index + 1
                                TaskOutputDetailData.sessionName = sessionName
                                onNavigateToTaskOutput()
                            },
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f)
                    ) {
                        Row(
                            modifier = Modifier.padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = if (success) Icons.Default.CheckCircle else Icons.Default.Error,
                                contentDescription = null,
                                modifier = Modifier.size(20.dp),
                                tint = if (success) Color(0xFF4CAF50) else Color(0xFFF44336)
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = task.toolCall.name,
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.Medium,
                                    fontFamily = FontFamily.Monospace
                                )
                                Text(
                                    text = "Duration: ${formatDuration3(task.durationMs)}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Icon(
                                imageVector = Icons.Default.ChevronRight,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskOutputDetailScreen(
    onNavigateBack: () -> Unit
) {
    val task = TaskOutputDetailData.task
    val taskNumber = TaskOutputDetailData.taskNumber
    val sessionName = TaskOutputDetailData.sessionName

    var searchText by remember { mutableStateOf("") }
    var searchMatches by remember { mutableIntStateOf(0) }
    var currentMatchIndex by remember { mutableIntStateOf(0) }

    // Build output lines
    val outputLines = remember(task) {
        val lines = mutableListOf<String>()
        task?.let { t ->
            lines.add("Tool: ${t.toolCall.name}")
            lines.add("Status: ${t.result.status}")
            lines.add("Duration: ${formatDuration3(t.durationMs)}")
            if (t.result.error != null) {
                lines.add("Error: ${t.result.error}")
            }
            t.result.value?.toString()?.let { value ->
                lines.addAll(value.split("\n"))
            }
        }
        lines
    }

    // Search logic
    LaunchedEffect(searchText) {
        if (searchText.isBlank()) {
            searchMatches = 0
            currentMatchIndex = 0
        } else {
            val matches = outputLines.filter { it.contains(searchText, ignoreCase = true) }.size
            searchMatches = matches
            currentMatchIndex = if (matches > 0) 1 else 0
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = sessionName.orEmpty(),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            text = task?.toolCall?.name ?: "Task Output",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.Default.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // Search bar
            OutlinedTextField(
                value = searchText,
                onValueChange = { searchText = it },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(12.dp),
                placeholder = { Text("Search output...") },
                leadingIcon = {
                    Icon(Icons.Default.Search, contentDescription = null)
                },
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { /* handled by LaunchedEffect */ })
            )

            if (searchText.isNotBlank()) {
                Row(
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = if (searchMatches > 0)
                            "Match $currentMatchIndex of $searchMatches"
                        else
                            "No matches found",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.weight(1f))
                    if (searchMatches > 0) {
                        IconButton(
                            onClick = {
                                currentMatchIndex = if (currentMatchIndex > 1) currentMatchIndex - 1 else searchMatches
                            },
                            modifier = Modifier.size(32.dp)
                        ) {
                            Icon(Icons.Default.KeyboardArrowUp, contentDescription = "Previous match")
                        }
                        IconButton(
                            onClick = {
                                currentMatchIndex = if (currentMatchIndex < searchMatches) currentMatchIndex + 1 else 1
                            },
                            modifier = Modifier.size(32.dp)
                        ) {
                            Icon(Icons.Default.KeyboardArrowDown, contentDescription = "Next match")
                        }
                    }
                }
            }

            // Output lines
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(12.dp)
            ) {
                outputLines.forEachIndexed { index, line ->
                    val isMatch = searchText.isNotBlank() &&
                            line.contains(searchText, ignoreCase = true)
                    Surface(
                        modifier = Modifier.fillMaxWidth(),
                        color = if (isMatch)
                            MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
                        else
                            Color.Transparent
                    ) {
                        Text(
                            text = line,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                            maxLines = 10,
                            overflow = TextOverflow.Ellipsis
                        )
                    }
                }
            }
        }
    }
}

private fun formatTimestamp2(millis: Long): String {
    val sdf = SimpleDateFormat("MMM d, yyyy h:mm a", Locale.getDefault())
    return sdf.format(Date(millis))
}

private fun formatDuration3(ms: Long): String {
    return when {
        ms < 1000 -> "${ms}ms"
        ms < 60_000 -> "${ms / 1000}.${(ms % 1000) / 100}s"
        else -> "${ms / 60_000}m ${(ms % 60_000) / 1000}s"
    }
}
