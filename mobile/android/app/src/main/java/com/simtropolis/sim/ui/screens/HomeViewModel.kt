package com.simtropolis.sim.ui.screens

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.simtropolis.sim.SimApplication
import com.simtropolis.sim.data.api.ApiResult
import com.simtropolis.sim.data.api.AgentRepository
import com.simtropolis.sim.data.api.SimApiService
import com.simtropolis.sim.data.api.SettingsRepository
import com.simtropolis.sim.data.model.AgentConfiguration
import com.simtropolis.sim.data.model.ChatSession
import com.simtropolis.sim.data.model.SessionInsights
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.Instant

data class HomeUiState(
    val sessions: List<ChatSession> = emptyList(),
    val isLoading: Boolean = false,
    val isLoadingMore: Boolean = false,
    val hasMoreSessions: Boolean = true,
    val isTrialMode: Boolean = true,
    val currentAgent: AgentConfiguration? = null,
    val insights: SessionInsights? = null,
    val isLoadingInsights: Boolean = false,
    val error: String? = null
)

class HomeViewModel : ViewModel() {
    private val TAG = "HomeViewModel"

    companion object {
        private const val INITIAL_DAYS_BACK = 5
        private const val LOAD_MORE_INCREMENT = 5
        private const val MAX_DAYS_TO_LOAD = 90
    }

    private val apiService: SimApiService = SimApplication.instance.apiService
    private val settingsRepository: SettingsRepository = SimApplication.instance.settingsRepository
    private val agentRepository: AgentRepository = SimApplication.instance.agentRepository

    private val _uiState = MutableStateFlow(HomeUiState())
    val uiState: StateFlow<HomeUiState> = _uiState.asStateFlow()

    // Pagination state
    private var currentDaysLoaded = INITIAL_DAYS_BACK
    private var allLoadedSessions: List<ChatSession> = emptyList()

    init {
        // Observe settings changes and reload sessions when baseUrl changes
        viewModelScope.launch {
            combine(
                settingsRepository.baseUrlFlow,
                agentRepository.currentAgentFlow
            ) { baseUrl, agent ->
                val isTrialMode = baseUrl.contains("demo-simed.fly.dev")
                _uiState.update {
                    it.copy(
                        isTrialMode = isTrialMode,
                        currentAgent = agent
                    )
                }
                Log.d(TAG, "Base URL changed, reloading sessions. Trial mode: $isTrialMode")
                resetAndLoadSessions()
                fetchInsights()
            }.collect()
        }
    }

    fun loadSessions() {
        resetAndLoadSessions()
    }

    fun refresh() {
        resetAndLoadSessions()
        fetchInsights()
    }

    fun fetchInsights() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingInsights = true) }
            val fallback = SessionInsights(totalSessions = 5, totalTokens = 450_000_000)
            val insights = when (val result = apiService.fetchInsights()) {
                is ApiResult.Success -> result.data
                is ApiResult.Error -> {
                    Log.e(TAG, "Failed to load insights: ${result.message}")
                    fallback
                }
            }
            _uiState.update {
                it.copy(
                    insights = insights,
                    isLoadingInsights = false
                )
            }
        }
    }

    fun loadMoreSessions() {
        if (_uiState.value.isLoadingMore || !_uiState.value.hasMoreSessions) return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoadingMore = true) }

            currentDaysLoaded += LOAD_MORE_INCREMENT
            if (currentDaysLoaded > MAX_DAYS_TO_LOAD) {
                currentDaysLoaded = MAX_DAYS_TO_LOAD
            }

            val filtered = filterSessionsByDaysBack(allLoadedSessions, currentDaysLoaded)

            _uiState.update {
                it.copy(
                    sessions = filtered,
                    isLoadingMore = false,
                    hasMoreSessions = currentDaysLoaded < MAX_DAYS_TO_LOAD
                            && filtered.size < allLoadedSessions.size
                )
            }
            Log.d(TAG, "Load more: days=$currentDaysLoaded, sessions=${filtered.size}")
        }
    }

    private fun resetAndLoadSessions() {
        currentDaysLoaded = INITIAL_DAYS_BACK
        allLoadedSessions = emptyList()

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, error = null) }

            when (val result = apiService.fetchSessions()) {
                is ApiResult.Success -> {
                    allLoadedSessions = result.data
                    val filtered = filterSessionsByDaysBack(result.data, currentDaysLoaded)
                    _uiState.update {
                        it.copy(
                            sessions = filtered,
                            isLoading = false,
                            hasMoreSessions = filtered.size < result.data.size
                                    || currentDaysLoaded < MAX_DAYS_TO_LOAD
                        )
                    }
                    Log.d(TAG, "Loaded ${result.data.size} sessions, showing ${filtered.size}")
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

    private suspend fun filterSessionsByDaysBack(
        sessions: List<ChatSession>,
        daysBack: Int
    ): List<ChatSession> = withContext(Dispatchers.Default) {
        if (daysBack >= MAX_DAYS_TO_LOAD) return@withContext sessions

        val cutoff = Instant.now().minusSeconds(daysBack * 86400L)
        sessions.filter { session ->
            try {
                val sessionDate = Instant.parse(session.updatedAt)
                sessionDate.isAfter(cutoff)
            } catch (_: Exception) {
                true // include sessions with unparseable dates
            }
        }
    }
}
