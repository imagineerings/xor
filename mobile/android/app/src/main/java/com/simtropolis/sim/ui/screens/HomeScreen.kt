package com.simtropolis.sim.ui.screens

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.launch
import com.simtropolis.sim.SimApplication
import com.simtropolis.sim.data.model.AgentConfiguration
import com.simtropolis.sim.data.model.ChatSession
import com.simtropolis.sim.ui.components.ChatInputView
import com.simtropolis.sim.ui.components.NodeFocus
import com.simtropolis.sim.ui.components.NodeMatrix
import com.simtropolis.sim.ui.components.WelcomeCard
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit

// ── Date-ordered session groups ──
private data class DateGroup(
    val header: String,
    val sessions: List<ChatSession>
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    onNavigateToChat: (String?, String?) -> Unit,
    onNavigateToSettings: () -> Unit,
    onNavigateToTrialInstructions: () -> Unit = onNavigateToSettings,
    viewModel: HomeViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    var inputText by remember { mutableStateOf("") }
    var showingSidebar by remember { mutableStateOf(false) }
    var selectedSession by remember { mutableStateOf<ChatSession?>(null) }
    var showSessionDetail by remember { mutableStateOf(false) }
    var selectedNodeSession by remember { mutableStateOf<ChatSession?>(null) }
    var daysOffset by remember { mutableStateOf(0) }

    // Favorites storage
    val favoritesStorage = remember { SimApplication.instance.favoriteSessionsStorage }
    val favoriteIds by favoritesStorage.favoriteIdsFlow.collectAsState(initial = emptySet())
    val scope = rememberCoroutineScope()

    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)

    LaunchedEffect(showingSidebar) {
        if (showingSidebar) drawerState.open() else drawerState.close()
    }

    LaunchedEffect(drawerState.currentValue) {
        showingSidebar = drawerState.currentValue == DrawerValue.Open
    }

    // Session detail dialog
    if (showSessionDetail && selectedSession != null) {
        AlertDialog(
            onDismissRequest = { showSessionDetail = false },
            confirmButton = {
                Button(onClick = {
                    showSessionDetail = false
                    onNavigateToChat(selectedSession!!.id, null)
                }) {
                    Text("Open Session")
                }
            },
            dismissButton = {
                TextButton(onClick = { showSessionDetail = false }) {
                    Text("Cancel")
                }
            },
            title = {
                Text(selectedSession!!.displayName)
            },
            text = {
                val session = selectedSession!!
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Schedule,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = formatTimestamp(session.updatedAt),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    if (!session.workingDir.isNullOrBlank()) {
                        Spacer(modifier = Modifier.height(6.dp))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Folder,
                                contentDescription = null,
                                modifier = Modifier.size(16.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = session.directoryName,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                    Spacer(modifier = Modifier.height(6.dp))
                    Text(
                        text = "${session.messageCount} message${if (session.messageCount == 1) "" else "s"}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        )
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet(
                modifier = Modifier.fillMaxWidth(0.85f)
            ) {
                SidebarContent(
                    sessions = uiState.sessions,
                    currentAgent = uiState.currentAgent,
                    favoriteIds = favoriteIds,
                    onToggleFavorite = { sessionId -> scope.launch { favoritesStorage.toggleFavorite(sessionId) } },
                    onSessionSelect = { sessionId ->
                        showingSidebar = false
                        onNavigateToChat(sessionId, null)
                    },
                    onNewSession = {
                        showingSidebar = false
                        onNavigateToChat(null, null)
                    },
                    onSettingsClick = {
                        showingSidebar = false
                        onNavigateToSettings()
                    }
                )
            }
        }
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
        ) {
            Column(
                modifier = Modifier.fillMaxSize()
            ) {
                // Welcome Card at top
                WelcomeCard(
                    onMenuClick = { showingSidebar = true },
                    tokenCount = uiState.insights?.totalTokens,
                    isLoadingTokens = uiState.isLoadingInsights,
                    sessions = sessionsForDayOffset(uiState.sessions, daysOffset),
                    daysOffset = daysOffset
                )

                // Sessions list or empty state
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                ) {
                    if (uiState.isLoading && uiState.sessions.isEmpty()) {
                        CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
                    } else {
                        Column(modifier = Modifier.fillMaxSize()) {
                            NodeMatrix(
                                sessions = uiState.sessions,
                                selectedSessionId = selectedNodeSession?.id,
                                onNodeTap = { session ->
                                    selectedNodeSession = session
                                },
                                onDayChange = { offset ->
                                    daysOffset = offset
                                    selectedNodeSession = null
                                },
                                favoriteIds = favoriteIds,
                                isLoading = uiState.isLoading,
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 16.dp, vertical = 8.dp)
                            )

                            selectedNodeSession?.let { session ->
                                NodeFocus(
                                    session = session,
                                    onContinueSession = {
                                        selectedNodeSession = null
                                        onNavigateToChat(it.id, null)
                                    },
                                    onDismiss = { selectedNodeSession = null },
                                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                                )
                            }

                            if (uiState.sessions.isEmpty()) {
                                Column(
                                    modifier = Modifier
                                        .weight(1f)
                                        .fillMaxWidth()
                                        .padding(32.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                    verticalArrangement = Arrangement.Center
                                ) {
                                    Text(
                                        text = "No recent sessions",
                                        style = MaterialTheme.typography.bodyLarge,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                    if (uiState.isTrialMode) {
                                        Spacer(modifier = Modifier.height(8.dp))
                                        Text(
                                            text = "Trial Mode",
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.primary
                                        )
                                    }
                                }
                            } else {
                                SessionsList(
                                    sessions = uiState.sessions,
                                    isLoadingMore = uiState.isLoadingMore,
                                    onSessionClick = { session ->
                                        selectedSession = session
                                        showSessionDetail = true
                                    },
                                    onLoadMore = { viewModel.loadMoreSessions() },
                                    modifier = Modifier.weight(1f)
                                )
                            }
                        }
                    }
                }

                // Trial mode banner
                if (uiState.isTrialMode) {
                    TrialModeBanner(
                        onClick = onNavigateToTrialInstructions
                    )
                }

                // Chat input at bottom
                ChatInputView(
                    text = inputText,
                    onTextChange = { inputText = it },
                    onSubmit = {
                        if (inputText.isNotBlank()) {
                            val message = inputText
                            inputText = ""
                            onNavigateToChat(null, message)
                        }
                    },
                    modifier = Modifier.padding(bottom = 16.dp)
                )
            }
        }
    }
}

// ── Sidebar ──

@Composable
private fun SidebarContent(
    sessions: List<ChatSession>,
    currentAgent: AgentConfiguration?,
    favoriteIds: Set<String>,
    onToggleFavorite: (String) -> Unit,
    onSessionSelect: (String) -> Unit,
    onNewSession: () -> Unit,
    onSettingsClick: () -> Unit
) {
    val favoriteSessions = remember(sessions, favoriteIds) {
        sessions.filter { favoriteIds.contains(it.id) }
    }
    val nonFavoriteSessions = remember(sessions, favoriteIds) {
        sessions.filter { !favoriteIds.contains(it.id) }
    }
    val nonFavoriteGroups = remember(nonFavoriteSessions) {
        groupSessionsByDate(nonFavoriteSessions)
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Sessions",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold
            )

            Row {
                IconButton(onClick = onNewSession) {
                    Icon(
                        imageVector = Icons.Default.Add,
                        contentDescription = "New Session"
                    )
                }
                IconButton(onClick = onSettingsClick) {
                    Icon(
                        imageVector = Icons.Default.Settings,
                        contentDescription = "Settings"
                    )
                }
            }
        }

        // Current agent indicator
        if (currentAgent != null) {
            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 8.dp),
                shape = RoundedCornerShape(8.dp),
                color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.HourglassEmpty,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = currentAgent.displayName,
                            style = MaterialTheme.typography.bodyMedium,
                            fontWeight = FontWeight.Medium
                        )
                    }
                }
            }
        }

        Divider(modifier = Modifier.padding(vertical = 8.dp))

        // Sessions list
        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            // Favorites section
            if (favoriteSessions.isNotEmpty()) {
                item {
                    DateSectionHeader("Favorites")
                }
                items(favoriteSessions, key = { it.id }) { session ->
                    SessionListItem(
                        session = session,
                        isFavorite = true,
                        onToggleFavorite = { onToggleFavorite(session.id) },
                        onClick = { onSessionSelect(session.id) }
                    )
                }
                item {
                    Divider(modifier = Modifier.padding(vertical = 4.dp))
                }
            }

            // Date-grouped non-favorite sessions
            for (group in nonFavoriteGroups) {
                item {
                    DateSectionHeader(group.header)
                }
                items(group.sessions, key = { it.id }) { session ->
                    SessionListItem(
                        session = session,
                        isFavorite = favoriteIds.contains(session.id),
                        onToggleFavorite = { onToggleFavorite(session.id) },
                        onClick = { onSessionSelect(session.id) }
                    )
                }
            }
        }
    }
}

@Composable
private fun DateSectionHeader(header: String) {
    Text(
        text = header,
        style = MaterialTheme.typography.labelLarge,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(vertical = 6.dp, horizontal = 4.dp)
    )
}

@Composable
private fun SessionListItem(
    session: ChatSession,
    isFavorite: Boolean,
    onToggleFavorite: () -> Unit,
    onClick: () -> Unit
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(8.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = session.displayName,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Spacer(modifier = Modifier.height(2.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = formatTimestamp(session.updatedAt),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    if (session.messageCount > 0) {
                        Text(
                            text = " · ${session.messageCount} msgs",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }

            // Favorite star toggle
            IconButton(
                onClick = onToggleFavorite,
                modifier = Modifier.size(32.dp)
            ) {
                Icon(
                    imageVector = if (isFavorite) Icons.Default.Star else Icons.Default.StarOutline,
                    contentDescription = if (isFavorite) "Unfavorite" else "Favorite",
                    modifier = Modifier.size(18.dp),
                    tint = if (isFavorite) Color(0xFFFFD700) else MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

// ── Sessions list (main home screen) ──

@Composable
private fun SessionsList(
    sessions: List<ChatSession>,
    isLoadingMore: Boolean = false,
    onSessionClick: (ChatSession) -> Unit,
    onLoadMore: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    val listState = rememberLazyListState()

    // Detect scroll to bottom to trigger pagination
    val shouldLoadMore = remember {
        derivedStateOf {
            val lastVisibleItem = listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            lastVisibleItem >= listState.layoutInfo.totalItemsCount - 3
        }
    }

    LaunchedEffect(shouldLoadMore.value) {
        if (shouldLoadMore.value && sessions.isNotEmpty()) {
            onLoadMore()
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        items(sessions, key = { it.id }) { session ->
            SessionCard(
                session = session,
                onClick = { onSessionClick(session) }
            )
        }

        if (isLoadingMore) {
            item {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp))
                }
            }
        }
    }
}

@Composable
private fun SessionCard(
    session: ChatSession,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Text(
                text = session.displayName,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Medium,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = formatTimestamp(session.updatedAt),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                if (session.messageCount > 0) {
                    Text(
                        text = " · ${session.messageCount} messages",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}

// ── Trial Mode Banner ──

@Composable
private fun TrialModeBanner(
    onClick: () -> Unit
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "🎯",
                style = MaterialTheme.typography.titleMedium
            )
            Spacer(modifier = Modifier.width(8.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = "Trial Mode",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Tap to connect your own sim instance",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

// ── Helpers ──

private fun groupSessionsByDate(sessions: List<ChatSession>): List<DateGroup> {
    val now = Instant.now()
    val today = now.atZone(ZoneId.systemDefault()).toLocalDate()
    val yesterday = today.minusDays(1)

    val groups = mutableListOf<DateGroup>()
    val todayList = mutableListOf<ChatSession>()
    val yesterdayList = mutableListOf<ChatSession>()
    val olderMap = mutableMapOf<String, MutableList<ChatSession>>()

    for (session in sessions) {
        try {
            val sessionDate = Instant.parse(session.updatedAt)
                .atZone(ZoneId.systemDefault()).toLocalDate()
            when {
                sessionDate == today -> todayList.add(session)
                sessionDate == yesterday -> yesterdayList.add(session)
                else -> {
                    val header = sessionDate.format(DateTimeFormatter.ofPattern("EEEE, MMM d"))
                    olderMap.getOrPut(header) { mutableListOf() }.add(session)
                }
            }
        } catch (_: Exception) {
            olderMap.getOrPut("Other") { mutableListOf() }.add(session)
        }
    }

    if (todayList.isNotEmpty()) groups.add(DateGroup("TODAY", todayList))
    if (yesterdayList.isNotEmpty()) groups.add(DateGroup("YESTERDAY", yesterdayList))

    // Older dates: most recent first
    val sortedOlderKeys = olderMap.keys.sortedByDescending { key ->
        try {
            val formatter = DateTimeFormatter.ofPattern("EEEE, MMM d")
            java.time.LocalDate.parse(key, formatter)
        } catch (_: Exception) {
            java.time.LocalDate.MIN
        }
    }
    for (key in sortedOlderKeys) {
        groups.add(DateGroup(key.uppercase(), olderMap[key]!!))
    }

    return groups
}

private fun formatTimestamp(isoString: String): String {
    return try {
        val instant = Instant.parse(isoString)
        val now = Instant.now()
        val minutes = ChronoUnit.MINUTES.between(instant, now)
        val hours = ChronoUnit.HOURS.between(instant, now)
        val days = ChronoUnit.DAYS.between(instant, now)

        when {
            minutes < 60 -> "$minutes minute${if (minutes == 1L) "" else "s"} ago"
            hours < 24 -> "$hours hour${if (hours == 1L) "" else "s"} ago"
            days == 1L -> "Yesterday"
            days < 7 -> "$days days ago"
            else -> {
                val formatter = DateTimeFormatter.ofPattern("MMM d")
                    .withZone(ZoneId.systemDefault())
                formatter.format(instant)
            }
        }
    } catch (e: Exception) {
        isoString
    }
}

private fun sessionsForDayOffset(sessions: List<ChatSession>, daysOffset: Int): List<ChatSession> {
    val target = java.time.LocalDate.now().minusDays(daysOffset.toLong())
    return sessions.filter { session ->
        try {
            Instant.parse(session.updatedAt).atZone(ZoneId.systemDefault()).toLocalDate() == target
        } catch (_: Exception) {
            daysOffset == 0
        }
    }
}
