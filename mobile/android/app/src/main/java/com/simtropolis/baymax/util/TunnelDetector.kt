package com.simtropolis.baymax.util

import android.content.Context
import android.content.Intent
import android.net.Uri

/**
 * Utility for detecting tunnel types from server URLs and building
 * deep link intents for tunnel management apps.
 *
 * Mirrors iOS TunnelDetector logic in ConfigurationHandler.
 */
object TunnelDetector {

    /**
     * Detect the tunnel type from a server URL.
     *
     * @param url The full server URL (e.g. https://100.123.45.67:62996)
     * @return The detected TunnelType
     */
    fun detectTunnelType(url: String): TunnelType {
        val lower = url.lowercase()

        return when {
            // Tailscale: 100.x.x.x IP or .ts.net domain
            lower.contains("100.") && (
                    lower.startsWith("http://100.") ||
                            lower.startsWith("https://100.") ||
                            lower.matches(Regex("https?://100\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}(:\\d+)?(/.*)?$"))
                    ) -> TunnelType.TAILSCALE

            lower.contains(".ts.net") -> TunnelType.TAILSCALE

            // Cloudflare tunnel proxy
            lower.contains("cloudflare-tunnel-proxy") ||
                    lower.contains(".trycloudflare.com") ||
                    lower.contains("cf-tunnel") -> TunnelType.CLOUDFLARE

            // Not a tunnel URL
            else -> TunnelType.NONE
        }
    }

    /**
     * Check if a URL points to a private network address.
     */
    fun isPrivateNetworkURL(url: String): Boolean {
        val host = try {
            Uri.parse(url).host ?: return false
        } catch (_: Exception) {
            return false
        }

        return host.startsWith("100.") ||
                host.startsWith("10.") ||
                host.startsWith("172.16.") ||
                host.startsWith("192.168.") ||
                host == "localhost" ||
                host == "127.0.0.1" ||
                host.endsWith(".local") ||
                host.endsWith(".ts.net")
    }

    /**
     * Build an intent to open the Tailscale app.
     * Falls back to Play Store if Tailscale is not installed.
     */
    fun openTailscaleApp(context: Context): Intent? {
        // Try Tailscale deep link first
        val tailscaleIntent = Intent(Intent.ACTION_VIEW).apply {
            data = Uri.parse("tailscale://")
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT
        }

        if (tailscaleIntent.resolveActivity(context.packageManager) != null) {
            return tailscaleIntent
        }

        // Fallback to Play Store
        return Intent(Intent.ACTION_VIEW).apply {
            data = Uri.parse("market://details?id=com.tailscale.ipn")
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
    }

    /**
     * Get a user-facing error message for a tunnel connection failure.
     */
    fun errorMessageForTunnel(url: String): String {
        val tunnelType = detectTunnelType(url)
        return when (tunnelType) {
            TunnelType.TAILSCALE -> "Please log in to Tailscale to connect to your agent"
            TunnelType.CLOUDFLARE -> "Could not reach your Cloudflare tunnel. Check that it's running."
            TunnelType.NONE -> {
                if (isPrivateNetworkURL(url)) {
                    "Cannot reach your agent on the local network. Is it running?"
                } else {
                    "Could not connect to server"
                }
            }
        }
    }
}
