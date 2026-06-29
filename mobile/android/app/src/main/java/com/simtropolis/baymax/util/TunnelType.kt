package com.simtropolis.baymax.util

/**
 * Detected tunnel type from a server URL.
 * Mirrors iOS TunnelDetector logic in ConfigurationHandler.
 */
enum class TunnelType {
    /** Not a tunnel URL — direct connection assumed. */
    NONE,

    /** Tailscale IP (100.x.x.x) or .ts.net domain. */
    TAILSCALE,

    /** Cloudflare tunnel proxy (lapstone / cloudflare-tunnel-proxy). */
    CLOUDFLARE
}
