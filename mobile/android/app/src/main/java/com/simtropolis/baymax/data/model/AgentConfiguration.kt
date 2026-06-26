package com.simtropolis.baymax.data.model

import kotlinx.serialization.Serializable

/**
 * Represents a saved agent/server configuration, matching iOS AgentConfiguration.
 */
@Serializable
data class AgentConfiguration(
    val id: String = java.util.UUID.randomUUID().toString(),
    val name: String? = null,
    val url: String,
    val secret: String,
    val lastUsed: Long = System.currentTimeMillis()
) {
    /** Display name: custom name, or a formatted/shortened URL */
    val displayName: String
        get() {
            if (!name.isNullOrBlank()) return name
            return defaultNameFor(url) ?: formatUrlForDisplay(url)
        }

    /** Subtitle showing the raw URL (only when a custom name is set) */
    val subtitle: String?
        get() {
            if (name.isNullOrBlank()) return null
            return formatUrlForDisplay(url)
        }

    companion object {
        /** Generate a default user-facing name based on URL patterns. */
        fun defaultNameFor(url: String): String? {
            val lower = url.lowercase()
            return when {
                lower.contains("demo-baymaxd") -> "Trial"
                lower.contains("cloudflare-tunnel-proxy") -> "Desktop"
                lower.contains("100.") || lower.contains(".ts.net") -> "Tailscale"
                lower.contains("localhost") || lower.contains("127.0.0.1") -> "Local"
                else -> null
            }
        }

        private fun formatUrlForDisplay(url: String): String {
            var formatted = url
                .removePrefix("https://")
                .removePrefix("http://")
                .removeSuffix(":443")
            if (formatted.length > 30) {
                formatted = formatted.take(27) + "..."
            }
            return formatted
        }
    }
}
