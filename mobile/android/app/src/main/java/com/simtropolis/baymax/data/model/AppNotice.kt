package com.simtropolis.baymax.data.model

/**
 * Represents a user-facing notice about connection or app state.
 * Mirrors iOS AppNoticeCenter / NoticeType.
 */
data class AppNotice(
    val type: NoticeType,
    val message: String,
    val action: NoticeAction? = null
)

enum class NoticeType {
    /** Configuration was applied successfully from a QR/deep link. */
    CONFIGURATION_SUCCESS,

    /** Server returned 503 — tunnel is disabled. */
    TUNNEL_DISABLED,

    /** Cannot connect to a private-network URL — tunnel may not be running. */
    TUNNEL_UNREACHABLE,

    /** Decoding failure — response format changed, app may be outdated. */
    APP_NEEDS_UPDATE
}

sealed class NoticeAction {
    /** Open another app (e.g. Tailscale) via its package name / URI. */
    data class OpenApp(val uri: String) : NoticeAction()

    /** Just dismiss the notice with no side-effect. */
    data object Dismiss : NoticeAction()
}
