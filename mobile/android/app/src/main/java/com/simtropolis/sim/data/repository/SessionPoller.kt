package com.simtropolis.sim.data.repository

import com.simtropolis.sim.SimApplication
import com.simtropolis.sim.data.api.ApiResult
import com.simtropolis.sim.data.model.Message
import com.simtropolis.sim.data.model.MessageContent
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Polls for new messages in a resumed session when SSE streaming is unavailable.
 *
 * Mirrors iOS session polling logic in ChatView:
 * - Poll interval: start at 2s, exponential backoff to 5s max
 * - Hash-based change detection
 * - Max 10 unchanged polls (~20s) then stop
 * - Stop on 404 (session deleted), user sends message, ViewModel cleared
 */
class SessionPoller {

    private var pollingJob: Job? = null
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private val _isPolling = MutableStateFlow(false)
    val isPolling: StateFlow<Boolean> = _isPolling.asStateFlow()

    private var _onMessagesUpdated: ((List<Message>) -> Unit)? = null

    /**
     * Start polling for updates on the given session.
     *
     * @param sessionId The session to poll.
     * @param initialMessages The current messages list (for change detection).
     * @param onMessagesUpdated Called on the main thread when new messages arrive.
     * @param onSessionDeleted Called when the session returns 404.
     */
    fun startPolling(
        sessionId: String,
        initialMessages: List<Message>,
        onMessagesUpdated: (List<Message>) -> Unit,
        onSessionDeleted: (() -> Unit)? = null
    ) {
        stopPolling()

        _isPolling.value = true
        _onMessagesUpdated = onMessagesUpdated

        var lastHash = messagesHash(initialMessages)
        var noChangeCount = 0
        var pollInterval = 2000L // Start at 2 seconds

        pollingJob = scope.launch {
            while (isActive && noChangeCount < 10) {
                delay(pollInterval)

                if (!isActive) break

                val apiService = SimApplication.instance.apiService

                try {
                    val result = apiService.resumeAgent(sessionId, loadModelAndExtensions = false)

                    when (result) {
                        is ApiResult.Success -> {
                            val newMessages = result.data.conversation ?: emptyList()
                            val newHash = messagesHash(newMessages)

                            if (newHash != lastHash) {
                                // Content changed
                                withContext(Dispatchers.Main) {
                                    onMessagesUpdated(newMessages)
                                }
                                noChangeCount = 0
                                lastHash = newHash
                                pollInterval = 2000L // Reset interval
                            } else {
                                noChangeCount++
                                // Exponential backoff up to 5 seconds
                                if (noChangeCount > 3) {
                                    pollInterval = (pollInterval * 1.3).toLong().coerceAtMost(5000L)
                                }
                            }
                        }

                        is ApiResult.Error -> {
                            // Check if it's a 404 (session deleted)
                            if (result.code == 404) {
                                withContext(Dispatchers.Main) {
                                    onSessionDeleted?.invoke()
                                }
                                break // Stop polling
                            }
                            // Other errors: increment no-change count
                            noChangeCount++
                            if (noChangeCount >= 10) break
                        }
                    }
                } catch (e: Exception) {
                    noChangeCount++
                    if (noChangeCount >= 10) break
                }
            }

            // Polling finished
            withContext(Dispatchers.Main) {
                _isPolling.value = false
            }
        }
    }

    /** Stop polling for updates. */
    fun stopPolling() {
        pollingJob?.cancel()
        pollingJob = null
        _isPolling.value = false
        _onMessagesUpdated = null
    }

    /** Generate a hash of messages for change detection. */
    private fun messagesHash(messages: List<Message>): String {
        val content = messages.joinToString(";") { msg ->
            val contentStr = msg.content.joinToString("|") { c ->
                when (c) {
                    is MessageContent.Text -> "t:${c.text}"
                    is MessageContent.ToolRequest -> "treq:${c.id}"
                    is MessageContent.ToolResponse -> "tres:${c.id}:${c.toolResult.status}"
                    is MessageContent.Thinking -> "think:${c.thinking.take(50)}"
                    else -> "other"
                }
            }
            "${msg.id}:${msg.role}:$contentStr"
        }
        val preview = content.take(100)
        return "${messages.count()}:${content.length}:$preview"
    }
}
