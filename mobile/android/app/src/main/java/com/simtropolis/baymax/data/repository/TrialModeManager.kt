package com.simtropolis.baymax.data.repository

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.simtropolis.baymax.BaymaxApplication
import com.simtropolis.baymax.data.api.ApiResult
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.runBlocking

private val Context.trialStore: DataStore<Preferences> by preferencesDataStore(name = "baymax_trial")

/**
 * Manages trial mode session persistence across app launches.
 * Mirrors iOS TrialMode.swift.
 *
 * - Persists the trial session ID in DataStore.
 * - On first launch, creates a new session via /agent/start.
 * - On subsequent launches, returns the existing session ID.
 */
class TrialModeManager(private val context: Context) {

    companion object {
        private val TRIAL_SESSION_ID_KEY = stringPreferencesKey("trial_session_id")
    }

    // ---- Synchronous accessor ----

    /** Returns the persisted trial session ID, or null if none. */
    val trialSessionId: String?
        get() = runBlocking {
            context.trialStore.data.first()[TRIAL_SESSION_ID_KEY]
        }

    // ---- Flow-based observer ----

    val trialSessionIdFlow: Flow<String?> = context.trialStore.data.map { prefs ->
        prefs[TRIAL_SESSION_ID_KEY]
    }

    // ---- Mutations ----

    /** Persist a trial session ID. */
    suspend fun saveTrialSessionId(sessionId: String) {
        context.trialStore.edit { prefs ->
            prefs[TRIAL_SESSION_ID_KEY] = sessionId
        }
    }

    /** Clear the persisted trial session ID. */
    suspend fun clearTrialSession() {
        context.trialStore.edit { prefs ->
            prefs.remove(TRIAL_SESSION_ID_KEY)
        }
    }

    /**
     * Get the existing trial session or create a new one.
     * Mirrors iOS TrialMode.getOrCreateTrialSession().
     */
    suspend fun getOrCreateTrialSession(): String {
        // Check for existing persisted session
        val existingId = context.trialStore.data.first()[TRIAL_SESSION_ID_KEY]
        if (existingId != null) {
            // Verify the session still exists server-side
            val apiService = BaymaxApplication.instance.apiService
            when (val result = apiService.resumeAgent(existingId, loadModelAndExtensions = false)) {
                is ApiResult.Success -> {
                    // Session is still valid
                    return existingId
                }

                is ApiResult.Error -> {
                    // Session may have been deleted — create a new one
                    clearTrialSession()
                }
            }
        }

        // Create a new session
        val apiService = BaymaxApplication.instance.apiService
        return when (val result = apiService.startAgent()) {
            is ApiResult.Success -> {
                val sessionId = result.data.id
                saveTrialSessionId(sessionId)
                sessionId
            }

            is ApiResult.Error -> {
                throw RuntimeException("Failed to create trial session: ${result.message}")
            }
        }
    }

    /** Whether the app is currently in trial mode. */
    val isTrial: Boolean
        get() {
            val repo = BaymaxApplication.instance.settingsRepository
            return repo.baseUrl.contains("demo-baymaxed.fly.dev")
        }
}
