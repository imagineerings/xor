package com.simtropolis.baymax.data.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
sealed class SSEEvent {
    @Serializable
    @SerialName("Message")
    data class MessageEvent(
        val type: String = "Message",
        val message: Message
    ) : SSEEvent()
    
    @Serializable
    @SerialName("Error")
    data class ErrorEvent(
        val type: String = "Error",
        val error: String
    ) : SSEEvent()
    
    @Serializable
    @SerialName("Finish")
    data class FinishEvent(
        val type: String = "Finish",
        val reason: String
    ) : SSEEvent()
    
    @Serializable
    @SerialName("ModelChange")
    data class ModelChangeEvent(
        val type: String = "ModelChange",
        val model: String,
        val mode: String
    ) : SSEEvent()
    
    @Serializable
    @SerialName("Ping")
    data class PingEvent(
        val type: String = "Ping"
    ) : SSEEvent()
    
    @Serializable
    @SerialName("UpdateConversation")
    data class UpdateConversationEvent(
        val type: String = "UpdateConversation",
        val conversation: List<Message>
    ) : SSEEvent()
}

@Serializable
data class ChatRequest(
    val messages: List<Message>,
    @SerialName("session_id")
    val sessionId: String,
    @SerialName("recipe_name")
    val recipeName: String? = null,
    @SerialName("recipe_version")
    val recipeVersion: String? = null
)
