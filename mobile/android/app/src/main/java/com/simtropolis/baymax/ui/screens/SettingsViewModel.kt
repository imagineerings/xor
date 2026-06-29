package com.simtropolis.baymax.ui.screens

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.simtropolis.baymax.BaymaxApplication
import com.simtropolis.baymax.data.api.AgentRepository
import com.simtropolis.baymax.data.api.ApiResult
import com.simtropolis.baymax.data.api.BaymaxApiService
import com.simtropolis.baymax.data.api.SettingsRepository
import com.simtropolis.baymax.data.model.AgentConfiguration
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

data class SettingsUiState(
    val baseUrl: String = SettingsRepository.DEFAULT_BASE_URL,
    val secretKey: String = SettingsRepository.DEFAULT_SECRET_KEY,
    val isConnected: Boolean = false,
    val isTesting: Boolean = false,
    val connectionError: String? = null,
    val savedAgents: List<AgentConfiguration> = emptyList(),
    val currentAgentId: String? = null
)

class SettingsViewModel : ViewModel() {
    private val TAG = "SettingsViewModel"

    private val apiService: BaymaxApiService = BaymaxApplication.instance.apiService
    private val settingsRepository: SettingsRepository = BaymaxApplication.instance.settingsRepository
    private val agentRepository: AgentRepository = BaymaxApplication.instance.agentRepository

    private val _uiState = MutableStateFlow(SettingsUiState())
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    init {
        loadSettings()
        observeAgents()
    }

    private fun loadSettings() {
        viewModelScope.launch {
            combine(
                settingsRepository.baseUrlFlow,
                settingsRepository.secretKeyFlow
            ) { baseUrl, secretKey ->
                _uiState.update {
                    it.copy(baseUrl = baseUrl, secretKey = secretKey)
                }
            }.collect()
        }
    }

    private fun observeAgents() {
        viewModelScope.launch {
            combine(
                agentRepository.savedAgentsFlow,
                agentRepository.currentAgentIdFlow
            ) { agents, currentId ->
                _uiState.update {
                    it.copy(
                        savedAgents = agents,
                        currentAgentId = currentId
                    )
                }
            }.collect()
        }
    }

    fun updateBaseUrl(url: String) {
        _uiState.update { it.copy(baseUrl = url) }
    }

    fun updateSecretKey(key: String) {
        _uiState.update { it.copy(secretKey = key) }
    }

    fun saveSettings() {
        viewModelScope.launch {
            settingsRepository.saveSettings(
                baseUrl = _uiState.value.baseUrl,
                secretKey = _uiState.value.secretKey
            )
            // Ensure this is in the agent list
            agentRepository.ensureCurrentAgentInList()
            Log.d(TAG, "Settings saved")
        }
    }

    fun testConnection() {
        viewModelScope.launch {
            _uiState.update { it.copy(isTesting = true, connectionError = null) }

            // Temporarily save settings for testing
            settingsRepository.saveSettings(
                baseUrl = _uiState.value.baseUrl,
                secretKey = _uiState.value.secretKey
            )

            when (val result = apiService.testConnection()) {
                is ApiResult.Success -> {
                    _uiState.update {
                        it.copy(
                            isConnected = true,
                            isTesting = false,
                            connectionError = null
                        )
                    }
                    // Save to agent list on successful connection
                    agentRepository.ensureCurrentAgentInList()
                    Log.d(TAG, "Connection test successful")
                }

                is ApiResult.Error -> {
                    // If test fails, restore original settings
                    val originalUrl = settingsRepository.baseUrl
                    val originalKey = settingsRepository.secretKey
                    settingsRepository.saveSettings(originalUrl, originalKey)

                    _uiState.update {
                        it.copy(
                            isConnected = false,
                            isTesting = false,
                            connectionError = result.message
                        )
                    }
                    Log.e(TAG, "Connection test failed: ${result.message}")
                }
            }
        }
    }

    /** Save the current input as a named agent. */
    fun saveCurrentAsAgent(name: String?) {
        viewModelScope.launch {
            val url = _uiState.value.baseUrl
            val secret = _uiState.value.secretKey

            // Check if this config already exists
            val existing = agentRepository.savedAgents.firstOrNull {
                it.url == url && it.secret == secret
            }
            if (existing != null) {
                val updated = existing.copy(name = name ?: existing.name)
                agentRepository.saveAgent(updated)
                agentRepository.switchToAgent(updated)
            } else {
                val agent = AgentConfiguration(
                    name = name ?: AgentConfiguration.defaultNameFor(url),
                    url = url,
                    secret = secret
                )
                agentRepository.saveAgent(agent)
                agentRepository.switchToAgent(agent)
            }
        }
    }

    /** Switch to a saved agent and test connection. */
    fun switchToAgent(agent: AgentConfiguration) {
        viewModelScope.launch {
            agentRepository.switchToAgent(agent)
            // Update local state
            _uiState.update {
                it.copy(
                    baseUrl = agent.url,
                    secretKey = agent.secret,
                    isConnected = false,
                    connectionError = null
                )
            }
            // Test the new connection
            when (val result = apiService.testConnection()) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isConnected = true) }
                }

                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            isConnected = false,
                            connectionError = result.message
                        )
                    }
                }
            }
        }
    }

    /** Delete a saved agent. */
    fun deleteAgent(agent: AgentConfiguration) {
        viewModelScope.launch {
            agentRepository.deleteAgent(agent.id)
        }
    }

    fun resetToTrialMode() {
        viewModelScope.launch {
            settingsRepository.resetToTrialMode()
            agentRepository.resetToTrial()
            _uiState.update {
                it.copy(
                    baseUrl = SettingsRepository.DEFAULT_BASE_URL,
                    secretKey = SettingsRepository.DEFAULT_SECRET_KEY,
                    isConnected = false,
                    connectionError = null
                )
            }
            Log.d(TAG, "Reset to trial mode")
        }
    }
}
