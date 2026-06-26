package com.simtropolis.baymax.util

import com.simtropolis.baymax.data.model.AppNotice
import com.simtropolis.baymax.data.model.NoticeAction
import com.simtropolis.baymax.data.model.NoticeType
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Singleton that manages user-facing app notices (connection issues, app updates).
 * Mirrors iOS AppNoticeCenter.
 *
 * Usage:
 *   NoticeManager.showNotice(AppNotice(...))
 *   NoticeManager.dismissNotice()
 *   val notice = NoticeManager.currentNotice.collect { ... }
 */
object NoticeManager {

    private val scope = CoroutineScope(Dispatchers.Main + Job())

    private val _currentNotice = MutableStateFlow<AppNotice?>(null)
    val currentNotice: StateFlow<AppNotice?> = _currentNotice.asStateFlow()

    private var dismissJob: Job? = null

    /** Show a notice. Auto-dismisses after 5 seconds. */
    fun showNotice(notice: AppNotice) {
        dismissJob?.cancel()
        _currentNotice.value = notice

        // Auto-dismiss after 5s for non-critical notices
        if (notice.action !is NoticeAction.OpenApp) {
            dismissJob = scope.launch {
                delay(5_000)
                _currentNotice.value = null
            }
        }
    }

    /** Dismiss the current notice. */
    fun dismissNotice() {
        dismissJob?.cancel()
        _currentNotice.value = null
    }

    /** Clear all notices immediately. */
    fun clearAll() {
        dismissJob?.cancel()
        _currentNotice.value = null
    }

    // Convenience methods for common notice types

    fun showTunnelDisabled() {
        showNotice(
            AppNotice(
                type = NoticeType.TUNNEL_DISABLED,
                message = "Tunnel appears to be disabled. Check your connection.",
                action = null
            )
        )
    }

    fun showTunnelUnreachable() {
        showNotice(
            AppNotice(
                type = NoticeType.TUNNEL_UNREACHABLE,
                message = "Cannot reach your agent. Is your tunnel running?",
                action = NoticeAction.OpenApp(
                    uri = "tailscale://"
                )
            )
        )
    }

    fun showAppNeedsUpdate() {
        showNotice(
            AppNotice(
                type = NoticeType.APP_NEEDS_UPDATE,
                message = "Response format changed. You may need to update the app.",
                action = null
            )
        )
    }

    /** Get the human-readable message for a TunnelType. */
    fun messageForTunnelType(tunnelType: TunnelType): String {
        return when (tunnelType) {
            TunnelType.TAILSCALE -> "Please log in to Tailscale to connect to your agent"
            TunnelType.CLOUDFLARE -> "Could not reach your Cloudflare tunnel"
            TunnelType.NONE -> "Could not connect to server"
        }
    }
}
