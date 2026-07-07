package com.simtropolis.sim.data.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ChatSession(
    val id: String,
    val description: String = "",
    @SerialName("message_count")
    val messageCount: Int = 0,
    @SerialName("created_at")
    val createdAt: String,
    @SerialName("updated_at")
    val updatedAt: String,
    @SerialName("working_dir")
    val workingDir: String? = null
) {
    val displayName: String
        get() = description.ifEmpty { "Session ${id.take(8)}" }

    val directoryName: String
        get() = workingDir?.substringAfterLast("/").orEmpty()
}

@Serializable
data class SessionsResponse(
    val sessions: List<ChatSession>
)

@Serializable
data class AgentResponse(
    val id: String,
    val conversation: List<Message>? = null
)

@Serializable
data class SessionResponse(
    val id: String,
    val conversation: List<Message>? = null
)

@Serializable
data class SessionInsights(
    @SerialName("total_sessions")
    val totalSessions: Int,
    @SerialName("total_tokens")
    val totalTokens: Long
)
