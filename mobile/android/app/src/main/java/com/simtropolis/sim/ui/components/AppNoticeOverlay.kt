package com.simtropolis.sim.ui.components

import android.content.Intent
import android.net.Uri
import androidx.compose.animation.*
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.sim.data.model.AppNotice
import com.simtropolis.sim.data.model.NoticeAction
import com.simtropolis.sim.data.model.NoticeType
import com.simtropolis.sim.util.NoticeManager

/**
 * Overlay composable that observes NoticeManager and shows a banner at the top.
 * Mirrors iOS AppNoticeOverlay.
 */
@Composable
fun AppNoticeOverlay(
    modifier: Modifier = Modifier
) {
    val notice by NoticeManager.currentNotice.collectAsState()
    val context = LocalContext.current

    AnimatedVisibility(
        visible = notice != null,
        enter = slideInVertically(initialOffsetY = { -it }) + fadeIn(),
        exit = slideOutVertically(targetOffsetY = { -it }) + fadeOut(),
        modifier = modifier
    ) {
        notice?.let { currentNotice ->
            AppNoticeBanner(
                notice = currentNotice,
                onDismiss = { NoticeManager.dismissNotice() },
                onAction = { action ->
                    when (action) {
                        is NoticeAction.OpenApp -> {
                            try {
                                val intent = Intent(Intent.ACTION_VIEW, Uri.parse(action.uri))
                                context.startActivity(intent)
                            } catch (_: Exception) {
                                // Fallback: if Tailscale not installed, open Play Store
                                if (action.uri.startsWith("tailscale://")) {
                                    try {
                                        val playStoreIntent = Intent(
                                            Intent.ACTION_VIEW,
                                            Uri.parse("market://details?id=com.tailscale.ipn")
                                        )
                                        context.startActivity(playStoreIntent)
                                    } catch (_: Exception) {
                                    }
                                }
                            }
                            NoticeManager.dismissNotice()
                        }

                        is NoticeAction.Dismiss -> {
                            NoticeManager.dismissNotice()
                        }
                    }
                }
            )
        }
    }
}

@Composable
private fun AppNoticeBanner(
    notice: AppNotice,
    onDismiss: () -> Unit,
    onAction: (NoticeAction) -> Unit
) {
    val backgroundColor = when (notice.type) {
        NoticeType.CONFIGURATION_SUCCESS -> Color(0xFF4CAF50)
        NoticeType.TUNNEL_DISABLED -> Color(0xFFF44336)  // Red
        NoticeType.TUNNEL_UNREACHABLE -> Color(0xFFFF9800)  // Orange
        NoticeType.APP_NEEDS_UPDATE -> Color(0xFF2196F3)  // Blue
    }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clickable {
                notice.action?.let { onAction(it) }
            },
        shape = RoundedCornerShape(bottomStart = 12.dp, bottomEnd = 12.dp),
        color = backgroundColor,
        tonalElevation = 4.dp
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Message text
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = when (notice.type) {
                        NoticeType.CONFIGURATION_SUCCESS -> "Configuration Saved"
                        NoticeType.TUNNEL_DISABLED -> "Tunnel Disabled"
                        NoticeType.TUNNEL_UNREACHABLE -> "Tunnel Unreachable"
                        NoticeType.APP_NEEDS_UPDATE -> "Update Available"
                    },
                    color = Color.White,
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 14.sp
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = notice.message,
                    color = Color.White.copy(alpha = 0.9f),
                    fontSize = 12.sp
                )
            }

            // Action hint
            if (notice.action != null) {
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "Open →",
                    color = Color.White,
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp
                )
            }

            // Dismiss button
            IconButton(
                onClick = onDismiss,
                modifier = Modifier.size(24.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Dismiss",
                    tint = Color.White.copy(alpha = 0.8f),
                    modifier = Modifier.size(16.dp)
                )
            }
        }
    }
}
