package com.simtropolis.sim.ui.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.sim.R
import com.simtropolis.sim.data.model.ChatSession
import com.simtropolis.sim.data.repository.ThemeManager
import kotlinx.coroutines.delay
import java.util.Calendar

@Composable
fun WelcomeCard(
    onMenuClick: () -> Unit,
    greeting: String? = null,
    tokenCount: Long? = null,
    isLoadingTokens: Boolean = false,
    sessions: List<ChatSession> = emptyList(),
    daysOffset: Int = 0,
    modifier: Modifier = Modifier
) {
    val isDark = ThemeManager.isDarkMode.value
    
    val baseGreeting = greeting ?: remember {
        val hour = Calendar.getInstance().get(Calendar.HOUR_OF_DAY)
        when {
            hour < 12 -> "Good morning!"
            hour < 17 -> "Good afternoon!"
            hour < 21 -> "Good evening!"
            else -> "Good night!"
        }
    }
    val densityGreeting = remember(sessions, daysOffset, baseGreeting) {
        if (daysOffset == 0) {
            baseGreeting
        } else {
            val dayLabel = if (daysOffset == 1) "yesterday" else "$daysOffset days ago"
            when (sessions.size) {
                in 0..2 -> "Quiet $dayLabel"
                in 3..5 -> "Light $dayLabel"
                else -> "Busy $dayLabel"
            }
        }
    }
    var displayedText by remember(densityGreeting) { mutableStateOf("") }

    LaunchedEffect(densityGreeting) {
        displayedText = ""
        for (index in densityGreeting.indices) {
            displayedText = densityGreeting.substring(0, index + 1)
            delay(20)
        }
    }
    
    val cardBackground = if (isDark) Color(0xFF1C1C1E) else Color.White
    
    Box(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(bottomStart = 32.dp, bottomEnd = 32.dp))
            .liquidGlassBackground(cardBackground)
            .padding(top = 48.dp, bottom = 32.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 24.dp)
        ) {
            // Sidebar toggle button (like iOS SideMenuIcon)
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                IconButton(onClick = onMenuClick) {
                    Icon(
                        imageVector = Icons.Default.Menu,
                        contentDescription = "Menu",
                        tint = MaterialTheme.colorScheme.onSurface
                    )
                }
                
                Spacer(modifier = Modifier.weight(1f))
            }
            
            Spacer(modifier = Modifier.height(24.dp))
            
            // Greeting text with sim logo (like iOS)
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top
            ) {
                Column(
                    modifier = Modifier.weight(1f)
                ) {
                    // Main greeting - 32px semibold like iOS
                    Text(
                        text = displayedText,
                        fontSize = 32.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Subheading - 16px regular with secondary color like iOS
                    Text(
                        text = "What do you want to do?",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Normal,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    Spacer(modifier = Modifier.height(16.dp))

                    TokenProgress(
                        tokenCount = tokenCount,
                        isLoading = isLoadingTokens
                    )
                }
                
                Spacer(modifier = Modifier.width(16.dp))
                
                // Sim logo on the right
                Image(
                    painter = painterResource(id = R.drawable.ic_sim_logo),
                    contentDescription = "Sim",
                    modifier = Modifier.size(48.dp),
                    colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface)
                )
            }
        }
    }
}

@Composable
private fun TokenProgress(
    tokenCount: Long?,
    isLoading: Boolean
) {
    val count = tokenCount ?: 0L
    val progress = (count.toFloat() / 1_000_000_000f).coerceIn(0f, 1f)

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Tokens",
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = if (isLoading) "Loading" else formatTokenCount(count),
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Spacer(modifier = Modifier.height(6.dp))
        LinearProgressIndicator(
            progress = if (isLoading) 0f else progress,
            modifier = Modifier
                .fillMaxWidth()
                .height(6.dp)
                .clip(RoundedCornerShape(3.dp)),
            color = MaterialTheme.colorScheme.primary,
            trackColor = MaterialTheme.colorScheme.surfaceVariant
        )
    }
}

fun formatTokenCount(count: Long): String {
    return when {
        count >= 1_000_000_000 -> "${count / 1_000_000_000}B"
        count >= 1_000_000 -> "${count / 1_000_000}M"
        count >= 1_000 -> "${count / 1_000}K"
        else -> count.toString()
    }
}
