package com.simtropolis.baymax.ui.screens

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.simtropolis.baymax.BaymaxApplication
import com.simtropolis.baymax.data.api.ApiResult
import com.simtropolis.baymax.data.api.AgentRepository
import com.simtropolis.baymax.data.api.BaymaxApiService
import com.simtropolis.baymax.data.api.SettingsRepository
import com.simtropolis.baymax.data.model.AgentConfiguration
import com.simtropolis.baymax.data.model.ChatSession
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

data class HomeUiState(
    val sessions: List<ChatSession> = emptyList(),
    val isLoading: Boolean = false,
    val isTrialMode: Boolean = true,
    val currentAgent: AgentConfiguration? = null,
    val error: String? = null
)

class HomeViewModel : ViewModel() {
    private val TAG = "HomeViewModel"

    private val apiService: BaymaxApiService = BaymaxApplication.instance.apiService
    private val settingsRepository: SettingsRepository = BaymaxApplication.instance.settingsRepository
    private val agentRepository: AgentRepository = BaymaxApplication.instance.agentRepository

    private val _uiState = MutableStateFlow(HomeUiState())
    val uiState: StateFlow<HomeUiState> = _uiState.asStateFlow()

    init {
        // Observe settings changes and reload sessions when baseUrl changes
        viewModelScope.launch {
            combine(
                settingsRepository.baseUrlFlow,
                agentRepository.currentAgentFlow
            ) { baseUrl, agent ->
                val isTrialMode = baseUrl.contains("demo-baymaxd.fly.dev")
                _uiState.update {
                    it.copy(
                        isTrialMode = isTrialMode,
                        currentAgent = agent
                    )
                }
                Log.d(TAG, "Base URL changed, reloading sessions. Trial mode: $isTrialMode")
                loadSessions()
            }.collect()
        }
    }

    fun loadSessions() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, error = null) }

            when (val result = apiService.fetchSessions()) {
                is ApiResult.Success -> {
                    _uiState.update {
                        it.copy(
                            sessions = result.data,
                            isLoading = false
                        )
                    }
                    Log.d(TAG, "Loaded ${result.data.size} sessions")
                }

                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            error = result.message
                        )
                    }
                    Log.e(TAG, "Failed to load sessions: ${result.message}")
                }
            }
        }
    }

    fun refresh() {
        loadSessions()
    }
}
