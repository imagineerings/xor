package com.simtropolis.baymax.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.simtropolis.baymax.data.model.Message
import com.simtropolis.baymax.data.model.MessageRole
import com.simtropolis.baymax.ui.components.AppNoticeOverlay
import com.simtropolis.baymax.ui.components.ChatInputView
import com.simtropolis.baymax.ui.components.CompletedToolCallData
import com.simtropolis.baymax.ui.components.FullTextOverlay
import com.simtropolis.baymax.ui.components.MessageBubble
import com.simtropolis.baymax.ui.components.StackedToolCallsView
import com.simtropolis.baymax.ui.components.ToolCallCard
import com.simtropolis.baymax.ui.components.ToolCallState
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.distinctUntilChanged

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(
    sessionId: String?,
    initialMessage: String? = null,
    onNavigateBack: () -> Unit,
    onNavigateToToolCallDetail: () -> Unit = {},
    onNavigateToTaskDetail: () -> Unit = {},
    viewModel: ChatViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    var inputText by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val coroutineScope = rememberCoroutineScope()
    var shouldAutoScroll by remember { mutableStateOf(true) }
    var userIsScrolling by remember { mutableStateOf(false) }
    var lastContentHeight by remember { mutableStateOf(0) }
    var lastScrollUpdate by remember { mutableStateOf(0L) }

    // Track if we've sent the initial message
    var hasProcessedInitialMessage by remember { mutableStateOf(false) }
    var showFullTextMessage by remember { mutableStateOf<Message?>(null) }

    // Load session or start new one
    LaunchedEffect(sessionId) {
        if (sessionId != null) {
            viewModel.loadSession(sessionId)
        } else {
            viewModel.startNewSession()
        }
    }

    // Auto-send initial message once we have a session ID (not waiting for full activation)
    // The sendMessage function handles session activation internally
    LaunchedEffect(uiState.currentSessionId, hasProcessedInitialMessage) {
        if (uiState.currentSessionId != null &&
            !hasProcessedInitialMessage &&
            !initialMessage.isNullOrBlank() &&
            !uiState.isLoadingSession
        ) {
            hasProcessedInitialMessage = true
            viewModel.sendMessage(initialMessage)
        }
    }

    LaunchedEffect(listState) {
        snapshotFlow { listState.isScrollInProgress }
            .distinctUntilChanged()
            .collect { scrolling ->
                userIsScrolling = scrolling
                if (!scrolling) {
                    val layoutInfo = listState.layoutInfo
                    val lastVisible = layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
                    shouldAutoScroll = lastVisible >= layoutInfo.totalItemsCount - 2
                } else {
                    shouldAutoScroll = false
                }
            }
    }

    LaunchedEffect(listState) {
        snapshotFlow { listState.layoutInfo.totalItemsCount }
            .collect { lastContentHeight = it }
    }

    val contentVersion = uiState.messages.joinToString("|") { message ->
        "${message.id}:${message.content.hashCode()}"
    }

    LaunchedEffect(contentVersion, lastContentHeight) {
        val lastIndex = listState.layoutInfo.totalItemsCount - 1
        val now = System.currentTimeMillis()
        if (lastIndex >= 0 && shouldAutoScroll && !userIsScrolling && now - lastScrollUpdate >= 100) {
            lastScrollUpdate = now
            coroutineScope.launch {
                listState.animateScrollToItem(lastIndex)
            }
        }
    }

    LaunchedEffect(uiState.isLoading) {
        if (!uiState.isLoading && uiState.messages.isNotEmpty()) {
            coroutineScope.launch {
                listState.animateScrollToItem((listState.layoutInfo.totalItemsCount - 1).coerceAtLeast(0))
            }
        }
    }

    Box(modifier = Modifier.fillMaxSize()) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = {
                        Text(
                            text = uiState.sessionName ?: "Chat",
                            style = MaterialTheme.typography.titleMedium
                        )
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
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .background(MaterialTheme.colorScheme.background)
            ) {
                Column(
                    modifier = Modifier.fillMaxSize()
                ) {
                    // App notice banner at top
                    AppNoticeOverlay()

                    // Messages list
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .fillMaxWidth()
                    ) {
                        when {
                            uiState.isLoadingSession -> {
                                // Loading session
                                Column(
                                    modifier = Modifier.align(Alignment.Center),
                                    horizontalAlignment = Alignment.CenterHorizontally
                                ) {
                                    CircularProgressIndicator()
                                    Spacer(modifier = Modifier.height(16.dp))
                                    Text(
                                        text = "Loading session...",
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }

                            uiState.messages.isEmpty() && !uiState.isActivatingSession && !uiState.isLoading -> {
                                // Empty state - show pending message if any
                                Column(
                                    modifier = Modifier
                                        .align(Alignment.Center)
                                        .padding(32.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally
                                ) {
                                    if (!initialMessage.isNullOrBlank() && !hasProcessedInitialMessage) {
                                        CircularProgressIndicator()
                                        Spacer(modifier = Modifier.height(16.dp))
                                        Text(
                                            text = "Preparing to send...",
                                            style = MaterialTheme.typography.bodyLarge,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    } else {
                                        Text(
                                            text = "Start a conversation...",
                                            style = MaterialTheme.typography.bodyLarge,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    }
                                }
                            }

                            else -> {
                                // Messages list
                                LazyColumn(
                                    state = listState,
                                    modifier = Modifier.fillMaxSize(),
                                    contentPadding = PaddingValues(vertical = 8.dp)
                                ) {
                                    // Group consecutive tool-only messages into clusters
                                    val renderedItems = buildMessageDisplayItems(
                                        messages = uiState.messages,
                                        groupedMessageIds = uiState.groupedToolCallMessages,
                                        activeToolCalls = uiState.activeToolCalls,
                                        completedToolCalls = uiState.completedToolCalls
                                    )

                                    items(
                                        items = renderedItems,
                                        key = { item ->
                                            when (item) {
                                                is DisplayItem.MessageItem -> "msg-" + item.message.id
                                                is DisplayItem.ToolCallItem -> "tc-" + item.toolCallState.id
                                                is DisplayItem.StackedToolCallsItem -> "stacked-" + item.messages.firstOrNull()?.id
                                                is DisplayItem.SpacerItem -> "spacer-" + item.height
                                            }
                                        }
                                    ) { item ->
                                        when (item) {
                                            is DisplayItem.MessageItem -> {
                                                MessageBubble(
                                                    message = item.message,
                                                    onLongPress = {
                                                        showFullTextMessage = item.message
                                                    }
                                                )
                                            }

                                            is DisplayItem.ToolCallItem -> {
                                                val state = item.toolCallState
                                                ToolCallCard(
                                                    toolCallState = state,
                                                    onClick = {
                                                        ToolCallDetailData.toolCallState = state
                                                        onNavigateToToolCallDetail()
                                                    },
                                                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                                                )
                                            }

                                            is DisplayItem.StackedToolCallsItem -> {
                                                StackedToolCallsView(
                                                    groupMessages = item.messages,
                                                    activeToolCalls = uiState.activeToolCalls,
                                                    completedToolCalls = uiState.completedToolCalls,
                                                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                                                )
                                            }

                                            is DisplayItem.SpacerItem -> {
                                                Spacer(modifier = Modifier.height(item.height.dp))
                                            }
                                        }
                                    }

                                    // Activating or thinking indicator
                                    if (uiState.isActivatingSession) {
                                        item {
                                            ActivatingIndicator()
                                        }
                                    } else if (uiState.isLoading) {
                                        item {
                                            ThinkingIndicator()
                                        }
                                    }

                                    // Polling indicator
                                    if (uiState.isPolling && !uiState.isLoading) {
                                        item {
                                            PollingIndicator()
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Error snackbar
                    uiState.error?.let { error ->
                        Snackbar(
                            modifier = Modifier.padding(16.dp),
                            action = {
                                TextButton(onClick = { viewModel.clearError() }) {
                                    Text("Dismiss")
                                }
                            }
                        ) {
                            Text(error)
                        }
                    }

                    // Gradient fade
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(40.dp)
                            .background(
                                brush = androidx.compose.ui.graphics.Brush.verticalGradient(
                                    colors = listOf(
                                        MaterialTheme.colorScheme.background.copy(alpha = 0f),
                                        MaterialTheme.colorScheme.background
                                    )
                                )
                            )
                    )

                    // Chat input
                    ChatInputView(
                        value = inputText,
                        onValueChange = { inputText = it },
                        onSend = {
                            if (inputText.isNotBlank()) {
                                viewModel.sendMessage(inputText)
                                inputText = ""
                            }
                        },
                        onStop = { viewModel.stopStreaming() },
                        isLoading = uiState.isLoading,
                        // Voice parameters
                        isListening = uiState.isListening,
                        voiceMode = uiState.voiceMode,
                        voiceState = uiState.voiceState,
                        onVoiceToggle = { viewModel.toggleVoiceInput() },
                        modifier = Modifier.padding(bottom = 16.dp)
                    )
                }
            }
        }

        // Full text overlay dialog
        showFullTextMessage?.let { message ->
            FullTextOverlay(
                message = message,
                onDismiss = { showFullTextMessage = null }
            )
        }
    }
}

/** Builds a flat list of display items from messages, handling tool call grouping. */
private data class DisplayId(val value: String)
private sealed class DisplayItem {
    data class MessageItem(val message: Message) : DisplayItem() {
        val id get() = "msg-${message.id}"
    }

    data class ToolCallItem(val toolCallState: ToolCallState) : DisplayItem() {
        val id get() = "tc-${toolCallState.id}"
    }

    data class StackedToolCallsItem(val messages: List<Message>) : DisplayItem() {
        val id get() = "stacked-${messages.firstOrNull()?.id ?: "empty"}"
    }

    data class SpacerItem(val height: Int) : DisplayItem() {
        val id get() = "spacer-$height"
    }
}

private fun buildMessageDisplayItems(
    messages: List<Message>,
    groupedMessageIds: Set<String>,
    activeToolCalls: Map<String, com.simtropolis.baymax.ui.components.ToolCallWithTiming>,
    completedToolCalls: Map<String, com.simtropolis.baymax.ui.components.CompletedToolCallData>
): List<DisplayItem> {
    val items = mutableListOf<DisplayItem>()

    // First pass: cluster consecutive grouped messages
    val groupedClusters = mutableListOf<List<Message>>()
    var currentCluster = mutableListOf<Message>()

    for (message in messages) {
        if (message.id in groupedMessageIds) {
            currentCluster.add(message)
        } else {
            if (currentCluster.isNotEmpty()) {
                groupedClusters.add(currentCluster.toList())
                currentCluster.clear()
            }
            // Check if this message has individual tool calls to show inline
            items.add(DisplayItem.MessageItem(message))
        }
    }
    if (currentCluster.isNotEmpty()) {
        groupedClusters.add(currentCluster.toList())
    }

    // Insert stacked tool call groups in their correct positions
    // We need to interleave clusters back into their original positions
    val result = mutableListOf<DisplayItem>()
    var msgIdx = 0
    var clusterIdx = 0

    for (message in messages) {
        if (groupedClusters.getOrNull(clusterIdx)?.contains(message) == true) {
            val cluster = groupedClusters[clusterIdx]
            result.add(DisplayItem.StackedToolCallsItem(cluster))
            clusterIdx++
            msgIdx += cluster.size
            // Skip messages that are part of the cluster
            while (msgIdx < messages.size && messages[msgIdx].id in groupedMessageIds) {
                msgIdx++
            }
        } else if (!(message.id in groupedMessageIds)) {
            // Find corresponding item from items list
            val existingItem = items.find { it is DisplayItem.MessageItem && it.message.id == message.id }
            if (existingItem != null) {
                result.add(existingItem)
            }
            msgIdx++
        }
    }

    // Ensure we got everything
    if (result.isEmpty()) {
        // Fallback: just add all non-tool-only messages
        for (message in messages) {
            if (message.hasNonEmptyTextContent) {
                result.add(DisplayItem.MessageItem(message))
            } else if (message.role == MessageRole.ASSISTANT) {
                // For assistant messages with only tool content, add tool call cards
                for (content in message.content) {
                    when (content) {
                        is com.simtropolis.baymax.data.model.MessageContent.ToolRequest -> {
                            val completed = completedToolCalls[content.id]
                            if (completed != null) {
                                result.add(
                                    DisplayItem.ToolCallItem(
                                        ToolCallState.Completed(
                                            id = content.id,
                                            toolCall = content.toolCall,
                                            result = completed.result,
                                            durationMs = completed.durationMs
                                        )
                                    )
                                )
                            } else {
                                val active = activeToolCalls[content.id]
                                if (active != null) {
                                    result.add(
                                        DisplayItem.ToolCallItem(
                                            ToolCallState.Active(
                                                id = content.id,
                                                toolCall = active.toolCall,
                                                startTime = active.startTime
                                            )
                                        )
                                    )
                                }
                            }
                        }

                        else -> {}
                    }
                }
            }
        }
    }

    return result
}

@Composable
private fun ActivatingIndicator() {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        CircularProgressIndicator(
            modifier = Modifier.size(16.dp),
            strokeWidth = 2.dp,
            color = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = "Preparing session...",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.primary
        )
    }
}

@Composable
private fun ThinkingIndicator() {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        CircularProgressIndicator(
            modifier = Modifier.size(16.dp),
            strokeWidth = 2.dp
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = "baymax is thinking...",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun PollingIndicator() {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        CircularProgressIndicator(
            modifier = Modifier.size(12.dp),
            strokeWidth = 2.dp,
            color = MaterialTheme.colorScheme.tertiary
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = "Checking for updates...",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}
