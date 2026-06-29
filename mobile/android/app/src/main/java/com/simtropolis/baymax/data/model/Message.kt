package com.simtropolis.baymax.data.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import java.util.UUID

@Serializable
data class Message(
    val id: String = UUID.randomUUID().toString(),
    val role: MessageRole,
    val content: List<MessageContent>,
    val created: Long = System.currentTimeMillis(),
    val metadata: MessageMetadata = MessageMetadata()
) {
    val hasNonEmptyTextContent: Boolean
        get() = content.any {
            it is MessageContent.Text && it.text.isNotBlank()
        }

    companion object {
        fun user(text: String) = Message(
            role = MessageRole.USER,
            content = listOf(MessageContent.Text(text = text))
        )

        fun assistant(text: String) = Message(
            role = MessageRole.ASSISTANT,
            content = listOf(MessageContent.Text(text = text))
        )
    }
}

@Serializable
enum class MessageRole {
    @SerialName("user")
    USER,
    @SerialName("assistant")
    ASSISTANT,
    @SerialName("system")
    SYSTEM
}

@Serializable
data class MessageMetadata(
    val userVisible: Boolean = true,
    val agentVisible: Boolean = true
)

// Use default "type" discriminator - matches server format
// SerialName values match the "type" field values from server
@Serializable
sealed class MessageContent {
    @Serializable
    @SerialName("text")
    data class Text(
        val text: String
    ) : MessageContent()

    @Serializable
    @SerialName("toolRequest")
    data class ToolRequest(
        val id: String,
        val toolCall: ToolCall
    ) : MessageContent()

    @Serializable
    @SerialName("toolResponse")
    data class ToolResponse(
        val id: String,
        val toolResult: ToolResult
    ) : MessageContent()

    @Serializable
    @SerialName("thinking")
    data class Thinking(
        val thinking: String,
        val signature: String
    ) : MessageContent()

    @Serializable
    @SerialName("toolConfirmationRequest")
    data class ToolConfirmationRequest(
        val id: String,
        @SerialName("tool_name")
        val toolName: String = "",
        val arguments: Map<String, JsonElement> = emptyMap()
    ) : MessageContent()

    @Serializable
    @SerialName("summarizationRequested")
    data class SummarizationRequested(
        val msg: String = ""
    ) : MessageContent()

    @Serializable
    @SerialName("conversationCompacted")
    data class ConversationCompacted(
        val msg: String
    ) : MessageContent()

    @Serializable
    @SerialName("systemNotification")
    data class SystemNotification(
        @SerialName("notification_type")
        val notificationType: String? = null,
        val msg: String
    ) : MessageContent()
}

@Serializable
data class ToolCall(
    val name: String,
    val arguments: Map<String, JsonElement> = emptyMap()
)

@Serializable
data class ToolResult(
    val status: String,
    val value: JsonElement? = null,
    val error: String? = null
)
