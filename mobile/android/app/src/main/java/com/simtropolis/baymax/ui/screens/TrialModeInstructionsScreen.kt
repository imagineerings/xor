package com.simtropolis.baymax.ui.screens

import android.content.Intent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.baymax.BaymaxApplication

private const val BAYMAX_DESKTOP_DOWNLOAD_URL =
    "https://github.com/block/baymax/releases/latest/download/Baymax.zip"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrialModeInstructionsScreen(
    onDismiss: () -> Unit,
    onNavigateToSettings: () -> Unit
) {
    val isDark = isSystemInDarkTheme()
    val context = LocalContext.current
    val scrollState = rememberScrollState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { },
                navigationIcon = {
                    IconButton(onClick = onDismiss) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Done"
                        )
                    }
                },
                actions = {
                    TextButton(onClick = onDismiss) {
                        Text(
                            text = "Done",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Medium
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .verticalScroll(scrollState)
                .padding(20.dp)
        ) {
            // Header
            Text(
                text = "Connect to Your Baymax Desktop",
                fontSize = 28.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "Follow these simple steps to link your phone to your desktop Baymax agent",
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Spacer(modifier = Modifier.height(24.dp))

            // Step 1: Install Baymax Desktop
            InstructionStep(
                number = 1,
                title = "Install Baymax Desktop",
                description = "Download and install the Baymax desktop application on your computer if you haven't already.",
                icon = "💻",
                isDark = isDark
            )

            // Download button
            Button(
                onClick = {
                    val intent = Intent(Intent.ACTION_SEND).apply {
                        type = "text/plain"
                        putExtra(Intent.EXTRA_TEXT, BAYMAX_DESKTOP_DOWNLOAD_URL)
                        putExtra(Intent.EXTRA_SUBJECT, "Download Baymax Desktop")
                    }
                    context.startActivity(
                        Intent.createChooser(intent, "Send Download Link")
                    )
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 48.dp, bottom = 20.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary
                ),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text(
                    text = "⬇ Send Download to Computer",
                    fontSize = 15.sp,
                    fontWeight = FontWeight.Medium
                )
            }

            // Step 2: Enable Tunneling
            InstructionStep(
                number = 2,
                title = "Enable Tunneling",
                description = "Open Baymax desktop, go to Settings → App, and turn on the Tunneling option.",
                icon = "🌐",
                isDark = isDark
            )

            Spacer(modifier = Modifier.height(20.dp))

            // Step 3: Scan QR Code
            InstructionStep(
                number = 3,
                title = "Scan QR Code",
                description = "A QR code will appear in the desktop app. Use your phone's camera to scan it.",
                icon = "📱",
                isDark = isDark
            )

            Spacer(modifier = Modifier.height(24.dp))

            // Important Note
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                color = if (isDark) Color(0x3326262B) else Color(0x1AF5F5FA),
                border = null
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = "ℹ️",
                            fontSize = 16.sp
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = "Important",
                            fontSize = 16.sp,
                            fontWeight = FontWeight.SemiBold,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Make sure both devices are connected to the internet. " +
                                "The tunneling feature creates a secure connection between " +
                                "your phone and desktop, allowing you to access your personal " +
                                "Baymax agent from anywhere.",
                        fontSize = 14.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            // Connect in Settings button
            OutlinedButton(
                onClick = onNavigateToSettings,
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text(
                    text = "Connect in Settings",
                    fontWeight = FontWeight.Medium
                )
            }

            Spacer(modifier = Modifier.height(24.dp))

            // Trial Mode Features
            Text(
                text = "Trial Mode Features",
                fontSize = 18.sp,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "While in trial mode, you can:",
                fontSize = 14.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(12.dp))

            FeatureRow(text = "Ask questions and get answers", isIncluded = true)
            FeatureRow(text = "Explore the app interface", isIncluded = true)
            FeatureRow(text = "Access your file system", isIncluded = false)
            FeatureRow(text = "Run commands and scripts", isIncluded = false)
            FeatureRow(text = "Save persistent sessions", isIncluded = false)

            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}

@Composable
private fun InstructionStep(
    number: Int,
    title: String,
    description: String,
    icon: String,
    isDark: Boolean
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(bottom = 8.dp),
        verticalAlignment = Alignment.Top
    ) {
        // Step number circle
        Box(
            modifier = Modifier
                .size(32.dp)
                .clip(CircleShape)
                .background(
                    if (isDark) Color(0xFF333338) else Color(0xFFF2F2F7)
                ),
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = "$number",
                fontSize = 14.sp,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface
            )
        }

        Spacer(modifier = Modifier.width(16.dp))

        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = icon,
                    fontSize = 16.sp
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = title,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }
            Text(
                text = description,
                fontSize = 14.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun FeatureRow(
    text: String,
    isIncluded: Boolean
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = if (isIncluded) "✅" else "❌",
            fontSize = 14.sp
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = text,
            fontSize = 14.sp,
            color = if (isIncluded) MaterialTheme.colorScheme.onSurface
            else MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}
