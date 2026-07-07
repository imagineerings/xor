package com.simtropolis.sim.ui.screens

import android.content.pm.ApplicationInfo
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.simtropolis.sim.SimApplication
import com.simtropolis.sim.data.model.AgentConfiguration
import com.simtropolis.sim.data.repository.ThemeManager

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onNavigateBack: () -> Unit,
    onNavigateToMarkdownTest: () -> Unit = {},
    viewModel: SettingsViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    var showResetDialog by remember { mutableStateOf(false) }
    var showSaveAgentDialog by remember { mutableStateOf(false) }
    var agentNameInput by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.Default.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                },
                actions = {
                    TextButton(
                        onClick = {
                            viewModel.saveSettings()
                            onNavigateBack()
                        }
                    ) {
                        Text("Save")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            // Server Configuration Section
            SettingsSection(title = "Server Configuration") {
                OutlinedTextField(
                    value = uiState.baseUrl,
                    onValueChange = { viewModel.updateBaseUrl(it) },
                    label = { Text("Base URL") },
                    placeholder = { Text("http://127.0.0.1:62996") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )

                Spacer(modifier = Modifier.height(12.dp))

                OutlinedTextField(
                    value = uiState.secretKey,
                    onValueChange = { viewModel.updateSecretKey(it) },
                    label = { Text("Secret Key") },
                    placeholder = { Text("Enter secret key") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation()
                )
            }

            // Connection Status Section
            SettingsSection(title = "Connection Status") {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            imageVector = if (uiState.isConnected) Icons.Default.CheckCircle else Icons.Default.Close,
                            contentDescription = null,
                            tint = if (uiState.isConnected) Color(0xFF4CAF50) else Color(0xFFF44336)
                        )
                        Column {
                            Text(
                                text = if (uiState.isConnected) "Connected" else "Disconnected",
                                style = MaterialTheme.typography.bodyLarge
                            )
                            uiState.connectionError?.let { error ->
                                Text(
                                    text = error,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.error
                                )
                            }
                        }
                    }

                    Button(
                        onClick = { viewModel.testConnection() },
                        enabled = !uiState.isTesting
                    ) {
                        if (uiState.isTesting) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(16.dp),
                                strokeWidth = 2.dp
                            )
                        } else {
                            Text("Test")
                        }
                    }
                }

                // Save Agent button
                if (uiState.isConnected) {
                    Spacer(modifier = Modifier.height(12.dp))
                    OutlinedButton(
                        onClick = {
                            // Pre-populate name if this agent already has one
                            val existing = uiState.savedAgents.firstOrNull {
                                it.url == uiState.baseUrl && it.secret == uiState.secretKey
                            }
                            agentNameInput = existing?.name ?: ""
                            showSaveAgentDialog = true
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(
                            imageVector = Icons.Default.Save,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Save Agent")
                    }
                }
            }

            // Saved Agents Section
            if (uiState.savedAgents.isNotEmpty()) {
                SettingsSection(title = "Saved Agents") {
                    uiState.savedAgents.forEach { agent ->
                        val isCurrent = agent.id == uiState.currentAgentId
                        AgentListItem(
                            agent = agent,
                            isCurrent = isCurrent,
                            onSwitch = { viewModel.switchToAgent(agent) },
                            onDelete = { viewModel.deleteAgent(agent) }
                        )
                        if (agent != uiState.savedAgents.last()) {
                            Divider(
                                modifier = Modifier.padding(vertical = 4.dp)
                            )
                        }
                    }
                }
            }

            SettingsSection(title = "Appearance") {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Dark Mode", style = MaterialTheme.typography.bodyLarge)
                    Switch(
                        checked = ThemeManager.isDarkMode.value,
                        onCheckedChange = { ThemeManager.setDarkMode(it) }
                    )
                }
            }

            if (isDebugBuild()) {
                SettingsSection(title = "Debug") {
                    OutlinedButton(
                        onClick = onNavigateToMarkdownTest,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(
                            imageVector = Icons.Default.Code,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Markdown Test")
                    }
                }
            }

            // About Section
            SettingsSection(title = "About") {
                Column {
                    Text(
                        text = "Sim",
                        style = MaterialTheme.typography.titleMedium
                    )
                    Text(
                        text = "A general purpose AI Agent by Simtropolis",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        text = "Version 1.0",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            // Reset Section
            SettingsSection(title = "") {
                OutlinedButton(
                    onClick = { showResetDialog = true },
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Icon(
                        imageVector = Icons.Default.Restore,
                        contentDescription = null,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Reset to Trial Mode")
                }
            }
        }
    }

    // Reset confirmation dialog
    if (showResetDialog) {
        AlertDialog(
            onDismissRequest = { showResetDialog = false },
            title = { Text("Reset to Trial Mode?") },
            text = { Text("This will reset your configuration to use the trial service.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        viewModel.resetToTrialMode()
                        showResetDialog = false
                    }
                ) {
                    Text("Reset", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showResetDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    // Save Agent dialog
    if (showSaveAgentDialog) {
        AlertDialog(
            onDismissRequest = { showSaveAgentDialog = false },
            title = { Text("Save Agent") },
            text = {
                OutlinedTextField(
                    value = agentNameInput,
                    onValueChange = { agentNameInput = it },
                    label = { Text("Agent Name (optional)") },
                    placeholder = { Text("e.g. Desktop, Trial, Work") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        viewModel.saveCurrentAsAgent(
                            name = agentNameInput.ifBlank { null }
                        )
                        showSaveAgentDialog = false
                        agentNameInput = ""
                    }
                ) {
                    Text("Save")
                }
            },
            dismissButton = {
                TextButton(onClick = { showSaveAgentDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }
}

private fun isDebugBuild(): Boolean {
    return SimApplication.instance.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
}

@Composable
private fun AgentListItem(
    agent: AgentConfiguration,
    isCurrent: Boolean,
    onSwitch: () -> Unit,
    onDelete: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Agent info
        Column(modifier = Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = agent.displayName,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = if (isCurrent) androidx.compose.ui.text.font.FontWeight.Bold else null
                )
                if (isCurrent) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "Active",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }
            agent.subtitle?.let { sub ->
                Text(
                    text = sub,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1
                )
            }
        }

        // Switch button
        if (!isCurrent) {
            TextButton(onClick = onSwitch) {
                Text("Switch")
            }
        }

        // Delete button
        IconButton(onClick = onDelete) {
            Icon(
                imageVector = Icons.Default.Delete,
                contentDescription = "Delete Agent",
                tint = MaterialTheme.colorScheme.error
            )
        }
    }
}

@Composable
private fun SettingsSection(
    title: String,
    content: @Composable ColumnScope.() -> Unit
) {
    Column {
        if (title.isNotEmpty()) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 8.dp)
            )
        }
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
            )
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                content = content
            )
        }
    }
}
