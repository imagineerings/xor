package com.simtropolis.baymax.ui.screens

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.simtropolis.baymax.BaymaxApplication
import com.simtropolis.baymax.data.api.ApiResult
import com.simtropolis.baymax.data.api.BaymaxApiService
import com.simtropolis.baymax.data.model.*
import com.simtropolis.baymax.data.repository.ContinuousVoiceManager
import com.simtropolis.baymax.data.repository.SessionPoller
import com.simtropolis.baymax.data.repository.VoiceManager
import com.simtropolis.baymax.data.repository.VoiceManagerCallback
import com.simtropolis.baymax.ui.components.CompletedToolCallData
import com.simtropolis.baymax.ui.components.ToolCallWithTiming
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

data class ChatUiState(
    val messages: List<Message> = emptyList(),
    val isLoading: Boolean = false,
    val isLoadingSession: Boolean = false,
    val isActivatingSession: Boolean = false,
    val currentSessionId: String? = null,
    val sessionName: String? = null,
    val isSessionActivated: Boolean = false,
    val error: String? = null,
    // Voice state
    val isListening: Boolean = false,
    val isSpeaking: Boolean = false,
    val voiceMode: VoiceMode = VoiceMode.Normal,
    val voiceTranscription: String = "",
    val voiceState: VoiceState = VoiceState(),
    // Polling state
    val isPolling: Boolean = false,
    // Tool call state
    val activeToolCalls: Map<String, ToolCallWithTiming> = emptyMap(),
    val completedToolCalls: Map<String, CompletedToolCallData> = emptyMap(),
    val groupedToolCallMessages: Set<String> = emptySet() // message IDs that are part of a tool-only group
)

class ChatViewModel : ViewModel() {
    private val TAG = "ChatViewModel"

    private val apiService: BaymaxApiService = BaymaxApplication.instance.apiService

    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private var streamJob: Job? = null

    // Memory limits
    companion object {
        const val MAX_MESSAGES = 50
        const val MAX_TOOL_CALLS = 20
    }

    // Session polling
    private val sessionPoller = SessionPoller()

    // Voice managers
    val voiceManager: VoiceManager by lazy {
        VoiceManager(BaymaxApplication.instance).apply {
            callback = voiceCallback
        }
    }
    val continuousVoiceManager: ContinuousVoiceManager by lazy {
        ContinuousVoiceManager(BaymaxApplication.instance).apply {
            callback = continuousVoiceCallback
        }
    }

    private val voiceCallback = object : VoiceManagerCallback {
        override fun onTranscriptionUpdate(partial: String) {
            _uiState.update { it.copy(voiceTranscription = partial) }
        }

        override fun onSubmitMessage(text: String) {
            _uiState.update { it.copy(voiceTranscription = text) }
            sendMessage(text)
        }

        override fun onCancelRequest() {
            stopStreaming()
        }
    }

    private val continuousVoiceCallback = object : VoiceManagerCallback {
        override fun onTranscriptionUpdate(partial: String) {
            _uiState.update { it.copy(voiceTranscription = partial) }
        }

        override fun onSubmitMessage(text: String) {
            _uiState.update { it.copy(voiceTranscription = text) }
            sendMessage(text)
        }

        override fun onCancelRequest() {
            stopStreaming()
        }
    }

    init {
        // Observe voice manager state
        viewModelScope.launch {
            voiceManager.state.collect { voiceState ->
                _uiState.update {
                    it.copy(
                        isListening = voiceState.isListening,
                        isSpeaking = voiceState.isSpeaking,
                        voiceMode = voiceState.mode,
                        voiceTranscription = voiceState.transcription,
                        voiceState = voiceState
                    )
                }
            }
        }
        viewModelScope.launch {
            continuousVoiceManager.state.collect { voiceState ->
                if (voiceState.mode == VoiceMode.Continuous) {
                    _uiState.update {
                        it.copy(
                            isListening = voiceState.isListening,
                            isSpeaking = voiceState.isSpeaking,
                            voiceTranscription = voiceState.transcription,
                            voiceState = voiceState
                        )
                    }
                }
            }
        }

        // Observe polling state
        viewModelScope.launch {
            sessionPoller.isPolling.collect { polling ->
                _uiState.update { it.copy(isPolling = polling) }
            }
        }

        // Start a periodic flush for streaming text batching
        viewModelScope.launch {
            while (true) {
                delay(33) // ~30fps
                flushTextBuffer()
            }
        }
    }

    /** Toggle voice input on/off. */
    fun toggleVoiceInput() {
        if (_uiState.value.isListening) {
            voiceManager.stopListening()
        } else {
            voiceManager.startListening()
        }
    }

    /** Toggle between Transcribe and Normal modes. */
    fun toggleVoiceMode() {
        voiceManager.mode = when (voiceManager.mode) {
            VoiceMode.Normal -> VoiceMode.Transcribe
            VoiceMode.Transcribe -> VoiceMode.Normal
            VoiceMode.Continuous -> VoiceMode.Normal
        }
    }

    fun startNewSession() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingSession = true, error = null) }

            when (val result = apiService.startAgent()) {
                is ApiResult.Success -> {
                    _uiState.update {
                        it.copy(
                            currentSessionId = result.data.id,
                            messages = result.data.conversation ?: emptyList(),
                            isLoadingSession = false,
                            isSessionActivated = false
                        )
                    }
                    Log.d(TAG, "Started new session: ${result.data.id}")
                }

                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            isLoadingSession = false,
                            error = result.message
                        )
                    }
                    Log.e(TAG, "Failed to start session: ${result.message}")
                }
            }
        }
    }

    fun loadSession(sessionId: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingSession = true, error = null) }

            when (val result = apiService.resumeAgent(sessionId, loadModelAndExtensions = false)) {
                is ApiResult.Success -> {
                    val loadedMessages = result.data.conversation ?: emptyList()
                    _uiState.update {
                        it.copy(
                            currentSessionId = result.data.id,
                            messages = loadedMessages,
                            isLoadingSession = false,
                            isSessionActivated = false
                        )
                    }
                    // Check if we should start polling for live updates
                    if (shouldPollForUpdates(loadedMessages)) {
                        Log.d(TAG, "Session appears to be waiting for response, starting polling...")
                        startPollingForUpdates()
                    }
                    Log.d(TAG, "Loaded session: ${result.data.id}")
                }

                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            isLoadingSession = false,
                            error = result.message
                        )
                    }
                    Log.e(TAG, "Failed to load session: ${result.message}")
                }
            }
        }
    }

    fun sendMessage(text: String) {
        val trimmedText = text.trim()
        if (trimmedText.isEmpty() || _uiState.value.isLoading) return

        val sessionId = _uiState.value.currentSessionId

        if (sessionId == null) {
            viewModelScope.launch {
                _uiState.update { it.copy(isLoadingSession = true) }

                when (val result = apiService.startAgent()) {
                    is ApiResult.Success -> {
                        _uiState.update {
                            it.copy(
                                currentSessionId = result.data.id,
                                isLoadingSession = false,
                                isSessionActivated = false
                            )
                        }
                        Log.d(TAG, "Created session: ${result.data.id}")
                        sendMessageToSession(trimmedText, result.data.id)
                    }

                    is ApiResult.Error -> {
                        _uiState.update {
                            it.copy(
                                isLoadingSession = false,
                                error = result.message
                            )
                        }
                    }
                }
            }
        } else {
            sendMessageToSession(trimmedText, sessionId)
        }
    }

    private fun sendMessageToSession(text: String, sessionId: String) {
        val userMessage = Message.user(text)

        _uiState.update { state ->
            state.copy(
                messages = state.messages + userMessage,
                isLoading = true,
                error = null
            )
        }

        streamJob = viewModelScope.launch {
            try {
                // Activate session if needed (matches iOS flow)
                if (!_uiState.value.isSessionActivated) {
                    _uiState.update { it.copy(isActivatingSession = true) }

                    Log.d(TAG, "Activating session: $sessionId")

                    // Resume agent with model and extensions
                    when (val resumeResult = apiService.resumeAgent(sessionId, loadModelAndExtensions = true)) {
                        is ApiResult.Success -> {
                            Log.d(TAG, "Resume agent successful")
                        }

                        is ApiResult.Error -> {
                            Log.e(TAG, "Resume agent failed: ${resumeResult.message}")
                        }
                    }

                    // Update from session (applies system prompt)
                    when (val updateResult = apiService.updateFromSession(sessionId)) {
                        is ApiResult.Success -> {
                            Log.d(TAG, "Update from session successful")
                        }

                        is ApiResult.Error -> {
                            Log.e(TAG, "Update from session failed: ${updateResult.message}")
                        }
                    }

                    _uiState.update {
                        it.copy(
                            isSessionActivated = true,
                            isActivatingSession = false
                        )
                    }
                }

                // Now stream the chat
                val allMessages = _uiState.value.messages
                Log.d(TAG, "Streaming chat with ${allMessages.size} messages")

                apiService.streamChat(allMessages, sessionId)
                    .catch { e ->
                        Log.e(TAG, "Stream error", e)
                        // Don't set error — switch to polling mode instead
                        _uiState.update {
                            it.copy(isLoading = false)
                        }
                        // Flush any remaining buffered text before switching to polling
                        flushTextBuffer()
                        // Start polling for updates in case the response completed server-side
                        startPollingForUpdates()
                    }
                    .collect { event ->
                        handleSSEEvent(event)
                    }

                // Flush any remaining buffered text
                flushTextBuffer()
                // After stream completes, check if we should poll for updates
                if (shouldPollForUpdates(_uiState.value.messages)) {
                    Log.d(TAG, "Session may still be processing, starting polling...")
                    startPollingForUpdates()
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error in sendMessageToSession", e)
                _uiState.update {
                    it.copy(
                        isLoading = false,
                        isActivatingSession = false,
                        error = e.message ?: "Failed to send message"
                    )
                }
            } finally {
                _uiState.update { it.copy(isLoading = false) }
            }
        }
    }

    private fun handleSSEEvent(event: SSEEvent) {
        when (event) {
            is SSEEvent.MessageEvent -> {
                _uiState.update { state ->
                    val existingIndex = state.messages.indexOfFirst { it.id == event.message.id }
                    val newMessages = if (existingIndex >= 0) {
                        // Accumulate streaming text content instead of replacing
                        state.messages.toMutableList().apply {
                            val existingMessage = this[existingIndex]
                            this[existingIndex] = accumulateMessageContent(existingMessage, event.message)
                        }
                    } else {
                        // Add new message
                        state.messages + event.message
                    }

                    // Enforce memory limits
                    val prunedMessages = limitMessages(newMessages)
                    // Rebuild tool call state from the updated messages
                    val toolCalls = rebuildToolCallState(prunedMessages)
                    val limitedToolCalls = limitToolCalls(toolCalls.second)
                    val grouped = findGroupedToolCallMessages(prunedMessages)
                    state.copy(
                        messages = prunedMessages,
                        activeToolCalls = toolCalls.first,
                        completedToolCalls = limitedToolCalls,
                        groupedToolCallMessages = grouped
                    )
                }
            }

            is SSEEvent.ErrorEvent -> {
                Log.e(TAG, "SSE Error: ${event.error}")
                _uiState.update {
                    it.copy(error = event.error)
                }
            }

            is SSEEvent.FinishEvent -> {
                Log.d(TAG, "Stream finished: ${event.reason}")
                _uiState.update { state ->
                    // Rebuild tool call state one final time
                    val toolCalls = rebuildToolCallState(state.messages)
                    state.copy(
                        isLoading = false,
                        activeToolCalls = toolCalls.first,
                        completedToolCalls = toolCalls.second
                    )
                }

                // Speak the last assistant response if voice is active
                speakLastResponse()
            }

            is SSEEvent.UpdateConversationEvent -> {
                Log.d(TAG, "Updating conversation with ${event.conversation.size} messages")
                _uiState.update {
                    val toolCalls = rebuildToolCallState(event.conversation)
                    val grouped = findGroupedToolCallMessages(event.conversation)
                    it.copy(
                        messages = event.conversation,
                        activeToolCalls = toolCalls.first,
                        completedToolCalls = toolCalls.second,
                        groupedToolCallMessages = grouped
                    )
                }
            }

            is SSEEvent.ModelChangeEvent -> {
                Log.d(TAG, "Model changed: ${event.model}")
            }

            is SSEEvent.PingEvent -> {
                // Ignore ping events
            }

            is SSEEvent.NotificationEvent -> {
                Log.d(TAG, "Notification: ${event.message.method}")
            }
        }
    }

    // ------------------------------------------------------------------
    // Memory Limits & Streaming Batching
    // ------------------------------------------------------------------

    /** Accumulated text buffer for streaming batching (30fps). */
    private val streamTextBuffer = mutableMapOf<String, String>()
    private var streamFlushJob: Job? = null

    /**
     * Prune oldest messages when exceeding MAX_MESSAGES.
     * Always preserves the first system message.
     */
    private fun limitMessages(messages: List<Message>): List<Message> {
        if (messages.size <= MAX_MESSAGES) return messages

        // Keep the first message (usually system) and the most recent MAX_MESSAGES-1
        val firstMessage = if (messages.first().role == MessageRole.SYSTEM) {
            listOf(messages.first())
        } else {
            emptyList()
        }

        val toKeep = messages.drop(firstMessage.size)
            .takeLast(MAX_MESSAGES - firstMessage.size)

        return firstMessage + toKeep
    }

    /**
     * Prune oldest completed tool calls when exceeding MAX_TOOL_CALLS.
     */
    private fun limitToolCalls(completedCalls: Map<String, CompletedToolCallData>): Map<String, CompletedToolCallData> {
        if (completedCalls.size <= MAX_TOOL_CALLS) return completedCalls

        return completedCalls.entries
            .sortedByDescending { it.value.completedAt }
            .take(MAX_TOOL_CALLS)
            .associate { it.key to it.value }
    }

    /**
     * Override accumulateMessageContent to add streaming text batching.
     * Instead of updating state on every SSE event, we buffer text and
     * flush at ~30fps to avoid choking the main thread.
     */
    private fun accumulateMessageContentWithBatching(existing: Message, incoming: Message): Message {
        val incomingText = incoming.content
            .filterIsInstance<MessageContent.Text>()
            .joinToString("") { it.text }

        if (incomingText.isEmpty()) {
            // Non-text update (tool request/response) — apply immediately
            val nonTextContent = incoming.content.filter { it !is MessageContent.Text }
            return existing.copy(content = (existing.content.filter { it is MessageContent.Text }) + nonTextContent)
        }

        // Buffer the incoming text
        val existingBuffer = streamTextBuffer[incoming.id] ?: ""
        streamTextBuffer[incoming.id] = existingBuffer + incomingText

        // Return existing message unchanged — the batched flush will update it
        return existing
    }

    /** Flush buffered streaming text to the UI state. */
    private fun flushTextBuffer() {
        if (streamTextBuffer.isEmpty()) return

        _uiState.update { state ->
            var newMessages = state.messages
            for ((messageId, bufferedText) in streamTextBuffer) {
                val index = newMessages.indexOfFirst { it.id == messageId }
                if (index >= 0 && bufferedText.isNotEmpty()) {
                    val existing = newMessages[index]
                    val existingText = existing.content
                        .filterIsInstance<MessageContent.Text>()
                        .joinToString("") { it.text }
                    val combined = existingText + bufferedText
                    val nonTextContent = existing.content.filter { it !is MessageContent.Text }
                    newMessages = newMessages.toMutableList().apply {
                        this[index] = existing.copy(
                            content = listOf(MessageContent.Text(text = combined)) + nonTextContent
                        )
                    }
                }
            }
            streamTextBuffer.clear()
            state.copy(messages = newMessages)
        }
    }

    // ------------------------------------------------------------------
    // Tool Call State Management
    // ------------------------------------------------------------------

    /**
     * Rebuild active and completed tool call maps from message content.
     * Mirrors iOS ChatView.rebuildToolCallState().
     */
    private fun rebuildToolCallState(messages: List<Message>): Pair<Map<String, ToolCallWithTiming>, Map<String, CompletedToolCallData>> {
        val active = mutableMapOf<String, ToolCallWithTiming>()
        val completed = mutableMapOf<String, CompletedToolCallData>()

        var assistantStartTime = System.currentTimeMillis()

        for (message in messages) {
            if (message.role == MessageRole.ASSISTANT) {
                for (content in message.content) {
                    when (content) {
                        is MessageContent.ToolRequest -> {
                            // Check if we have a matching response
                            val hasResponse = messages.any { msg ->
                                msg.content.any { c ->
                                    c is MessageContent.ToolResponse && c.id == content.id
                                }
                            }
                            if (hasResponse) {
                                // Will be processed as completed when we hit the response
                                active[content.id] = ToolCallWithTiming(
                                    toolCall = content.toolCall,
                                    startTime = assistantStartTime
                                )
                            } else {
                                // No response yet — still active
                                active[content.id] = ToolCallWithTiming(
                                    toolCall = content.toolCall,
                                    startTime = assistantStartTime
                                )
                            }
                        }

                        is MessageContent.ToolResponse -> {
                            // Find matching request
                            val requestContent = message.content
                                .filterIsInstance<MessageContent.ToolRequest>()
                                .firstOrNull { it.id == content.id }

                            val timing = active.remove(content.id)
                            if (timing != null) {
                                val durationMs = System.currentTimeMillis() - timing.startTime
                                completed[content.id] = CompletedToolCallData(
                                    toolCall = timing.toolCall,
                                    result = content.toolResult,
                                    durationMs = durationMs
                                )
                            } else if (requestContent != null) {
                                // Timing not tracked (e.g. polled data), use estimate
                                completed[content.id] = CompletedToolCallData(
                                    toolCall = requestContent.toolCall,
                                    result = content.toolResult,
                                    durationMs = 0
                                )
                            }
                        }

                        else -> {}
                    }
                }
            }
            if (message.role == MessageRole.ASSISTANT) {
                assistantStartTime = System.currentTimeMillis()
            }
        }

        return Pair(active, completed)
    }

    /**
     * Find message IDs that are part of consecutive tool-only assistant messages.
     * These should be grouped visually.
     * Mirrors iOS ChatView.groupConsecutiveToolOnlyMessages().
     */
    private fun findGroupedToolCallMessages(messages: List<Message>): Set<String> {
        val grouped = mutableSetOf<String>()
        var i = 0
        while (i < messages.size) {
            if (isToolOnlyMessage(messages[i])) {
                // Start of a potential group
                val groupStart = i
                while (i < messages.size && isToolOnlyMessage(messages[i]) && !hasUserMessageBetween(
                        messages,
                        groupStart,
                        i
                    )
                ) {
                    i++
                }
                // groupStart..<i is a consecutive block of tool-only assistant messages
                if (i - groupStart > 1) {
                    // Only group if there are at least 2 consecutive
                    for (j in groupStart until i) {
                        grouped.add(messages[j].id)
                    }
                }
            } else {
                i++
            }
        }
        return grouped
    }

    private fun isToolOnlyMessage(message: Message): Boolean {
        if (message.role != MessageRole.ASSISTANT) return false
        // A tool-only message has no text content — only tool requests/responses
        val hasText = message.content.any { it is MessageContent.Text && it.text.isNotBlank() }
        val hasToolContent = message.content.any {
            it is MessageContent.ToolRequest || it is MessageContent.ToolResponse
        }
        return !hasText && hasToolContent
    }

    private fun hasUserMessageBetween(messages: List<Message>, start: Int, end: Int): Boolean {
        for (i in start until end) {
            if (messages[i].role == MessageRole.USER) return true
        }
        return false
    }

    /** Speak the last assistant response via TTS. */
    private fun speakLastResponse() {
        val lastMessage = _uiState.value.messages.lastOrNull()
        if (lastMessage?.role == MessageRole.ASSISTANT) {
            val text = lastMessage.content
                .filterIsInstance<MessageContent.Text>()
                .joinToString("") { it.text }
            if (text.isNotBlank()) {
                // Check which voice mode is active
                if (voiceManager.mode == VoiceMode.Transcribe) {
                    voiceManager.speakResponse(text)
                } else if (continuousVoiceManager.isVoiceMode) {
                    continuousVoiceManager.speakResponse(text)
                }
            } else {
                // If no assistant text, ensure continuous returns to listening
                if (continuousVoiceManager.isVoiceMode) {
                    continuousVoiceManager.onProcessingComplete()
                }
            }
        }
    }

    /**
     * Accumulate streaming content - appends new text to existing text content
     */
    private fun accumulateMessageContent(existing: Message, incoming: Message): Message {
        // Use the batched version for streaming performance
        val result = accumulateMessageContentWithBatching(existing, incoming)

        // Schedule periodic flush at ~30fps
        if (streamFlushJob?.isActive != true) {
            streamFlushJob = viewModelScope.launch {
                delay(33) // ~30fps
                flushTextBuffer()
            }
        }

        return result
    }

    fun stopStreaming() {
        streamJob?.cancel()
        streamJob = null
        streamFlushJob?.cancel()
        streamFlushJob = null
        streamTextBuffer.clear()
        stopPolling()
        _uiState.update { it.copy(isLoading = false) }
    }

    // ------------------------------------------------------------------
    // Session Polling
    // ------------------------------------------------------------------

    /**
     * Start polling when SSE streaming fails but the session is still active.
     */
    fun startPollingForUpdates() {
        val sessionId = _uiState.value.currentSessionId ?: return
        val messages = _uiState.value.messages

        sessionPoller.startPolling(
            sessionId = sessionId,
            initialMessages = messages,
            onMessagesUpdated = { newMessages ->
                _uiState.update {
                    it.copy(messages = newMessages)
                }
            },
            onSessionDeleted = {
                Log.d(TAG, "Session $sessionId no longer exists, stopping polling")
            }
        )
    }

    /** Stop polling for updates. */
    fun stopPolling() {
        sessionPoller.stopPolling()
    }

    /**
     * Check whether we should start polling based on the last message state.
     * Mirrors iOS ChatView.shouldPollForUpdates().
     */
    private fun shouldPollForUpdates(messages: List<Message>): Boolean {
        val lastMessage = messages.lastOrNull() ?: return false

        // created timestamp - detect if it's in seconds or milliseconds
        val createdTimestamp = lastMessage.created
        val createdDate: java.util.Date = if (createdTimestamp > 4_102_444_800) {
            java.util.Date(createdTimestamp)
        } else {
            java.util.Date(createdTimestamp * 1000)
        }

        val age = System.currentTimeMillis() - createdDate.time
        val ageSeconds = age / 1000

        // Only poll if message is recent (< 2 minutes)
        if (ageSeconds !in -60..120) return false

        // Check if we're waiting for a response
        val isWaitingForResponse = lastMessage.role == MessageRole.USER
        return isWaitingForResponse
    }

    /** Refresh the current session (for pull-to-refresh). */
    fun refreshCurrentSession() {
        val sessionId = _uiState.value.currentSessionId ?: return

        viewModelScope.launch {
            when (val result = apiService.resumeAgent(sessionId, loadModelAndExtensions = false)) {
                is ApiResult.Success -> {
                    val newMessages = result.data.conversation ?: emptyList()
                    _uiState.update {
                        it.copy(messages = newMessages)
                    }
                    Log.d(TAG, "Session refreshed with ${newMessages.size} messages")
                }

                is ApiResult.Error -> {
                    Log.e(TAG, "Failed to refresh session: ${result.message}")
                }
            }
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }

    override fun onCleared() {
        super.onCleared()
        streamJob?.cancel()
        streamFlushJob?.cancel()
        streamTextBuffer.clear()
        voiceManager.release()
        continuousVoiceManager.release()
    }
}
