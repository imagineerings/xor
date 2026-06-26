package com.simtropolis.baymax.util

import android.content.Intent
import android.net.Uri
import com.simtropolis.baymax.BaymaxApplication
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.json.JSONObject

/**
 * Handles deep-link configuration from QR codes.
 *
 * Expected format:
 *   baymaxchat://configure?data=<url-encoded-json>
 * where the JSON payload is:
 *   {"url": "...", "secret": "..."}
 *
 * Mirrors iOS ConfigurationHandler.
 */
object QRConfigHandler {

    private val scope = CoroutineScope(Dispatchers.IO)

    /**
     * Attempt to parse and apply a deep-link configuration URI.
     *
     * @param uri The incoming URI (e.g. from Intent.data).
     * @param onResult Called on the calling thread with (success, errorMessage).
     */
    fun handleDeepLink(uri: Uri, onResult: ((Boolean, String?) -> Unit)? = null) {
        // Expected: baymaxchat://configure?data=<encoded>
        if (uri.host != "configure" || uri.getQueryParameter("data") == null) {
            onResult?.invoke(false, "Invalid configuration link: missing 'data' parameter")
            return
        }

        val encodedData = uri.getQueryParameter("data") ?: run {
            onResult?.invoke(false, "Missing 'data' parameter")
            return
        }

        // URL decode
        val decoded: String = try {
            java.net.URLDecoder.decode(encodedData, "UTF-8")
        } catch (e: Exception) {
            onResult?.invoke(false, "Failed to decode configuration data")
            return
        }

        // Parse JSON
        val (url, secret) = try {
            val json = JSONObject(decoded)
            val rawUrl = json.optString("url", "").trim()
            val rawSecret = json.optString("secret", "").trim()
            if (rawUrl.isEmpty() || rawSecret.isEmpty()) {
                onResult?.invoke(false, "Configuration missing 'url' or 'secret'")
                return
            }
            Pair(rawUrl, rawSecret)
        } catch (e: Exception) {
            onResult?.invoke(false, "Invalid configuration format: ${e.message}")
            return
        }

        // Normalize URL
        val normalizedUrl = normalizeUrl(url)

        // Apply to SettingsRepository
        val repo = BaymaxApplication.instance.settingsRepository
        scope.launch {
            repo.saveSettings(normalizedUrl, secret)

            // Test connection
            val apiService = BaymaxApplication.instance.apiService
            val connected = apiService.testConnection()

            if (connected is com.simtropolis.baymax.data.api.ApiResult.Success && connected.data) {
                onResult?.invoke(true, null)
            } else {
                val errorMsg = when (connected) {
                    is com.simtropolis.baymax.data.api.ApiResult.Error -> connected.message
                    else -> "Connection test failed"
                }

                // Tailscale-specific error
                val finalError = if (isTailscaleUrl(normalizedUrl)) {
                    "Please log in to Tailscale to connect to your agent"
                } else {
                    errorMsg
                }
                onResult?.invoke(false, finalError)
            }
        }
    }

    /** Normalize a server URL: add https:// if missing, strip :443 port. */
    fun normalizeUrl(raw: String): String {
        val withScheme = if (raw.startsWith("http://") || raw.startsWith("https://")) {
            raw
        } else {
            "https://$raw"
        }
        return withScheme.removeSuffix(":443")
    }

    /** Detect whether a URL is a Tailscale address (100.x.x.x or .ts.net). */
    fun isTailscaleUrl(url: String): Boolean {
        return url.startsWith("http://100.") ||
                url.startsWith("https://100.") ||
                url.contains(".ts.net")
    }
}
