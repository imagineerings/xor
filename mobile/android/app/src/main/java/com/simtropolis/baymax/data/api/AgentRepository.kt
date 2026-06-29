package com.simtropolis.baymax.data.api

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.simtropolis.baymax.data.model.AgentConfiguration
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private val Context.agentStore: DataStore<Preferences> by preferencesDataStore(name = "baymax_agents")

/**
 * Manages saved agent/server configurations.
 * Mirrors iOS AgentStorage in ConfigurationHandler.swift.
 */
class AgentRepository(private val context: Context) {

    companion object {
        private val AGENTS_KEY = stringPreferencesKey("saved_agents")
        private val CURRENT_AGENT_ID_KEY = stringPreferencesKey("current_agent_id")

        private val json = Json { ignoreUnknownKeys = true }
    }

    // ---- Synchronous accessors (for quick reads) ----

    val savedAgents: List<AgentConfiguration>
        get() = runBlocking {
            val raw = context.agentStore.data.first()[AGENTS_KEY] ?: return@runBlocking emptyList()
            try {
                json.decodeFromString<List<AgentConfiguration>>(raw)
                    .sortedByDescending { it.lastUsed }
            } catch (_: Exception) {
                emptyList()
            }
        }

    val currentAgentId: String?
        get() = runBlocking {
            context.agentStore.data.first()[CURRENT_AGENT_ID_KEY]
        }

    val currentAgent: AgentConfiguration?
        get() {
            val id = currentAgentId ?: return null
            return savedAgents.firstOrNull { it.id == id }
        }

    // ---- Flow-based observers ----

    val savedAgentsFlow: Flow<List<AgentConfiguration>> = context.agentStore.data.map { prefs ->
        val raw = prefs[AGENTS_KEY] ?: return@map emptyList()
        try {
            json.decodeFromString<List<AgentConfiguration>>(raw)
                .sortedByDescending { it.lastUsed }
        } catch (_: Exception) {
            emptyList()
        }
    }

    val currentAgentIdFlow: Flow<String?> = context.agentStore.data.map { prefs ->
        prefs[CURRENT_AGENT_ID_KEY]
    }

    val currentAgentFlow: Flow<AgentConfiguration?> = context.agentStore.data.map { prefs ->
        val id = prefs[CURRENT_AGENT_ID_KEY] ?: return@map null
        val raw = prefs[AGENTS_KEY] ?: return@map null
        try {
            val agents = json.decodeFromString<List<AgentConfiguration>>(raw)
            agents.firstOrNull { it.id == id }
        } catch (_: Exception) {
            null
        }
    }

    // ---- Mutations ----

    /** Add or update an agent configuration. */
    suspend fun saveAgent(agent: AgentConfiguration) {
        context.agentStore.edit { prefs ->
            val raw = prefs[AGENTS_KEY] ?: "[]"
            val agents = try {
                json.decodeFromString<MutableList<AgentConfiguration>>(raw)
            } catch (_: Exception) {
                mutableListOf()
            }

            val index = agents.indexOfFirst { it.id == agent.id }
            val updated = agent.copy(lastUsed = System.currentTimeMillis())

            if (index >= 0) {
                agents[index] = updated
            } else {
                agents.add(0, updated)
            }

            prefs[AGENTS_KEY] = json.encodeToString(agents.sortedByDescending { it.lastUsed })
        }
    }

    /** Delete an agent by ID. */
    suspend fun deleteAgent(id: String) {
        context.agentStore.edit { prefs ->
            val raw = prefs[AGENTS_KEY] ?: return@edit
            val agents = try {
                json.decodeFromString<MutableList<AgentConfiguration>>(raw)
            } catch (_: Exception) {
                return@edit
            }

            agents.removeAll { it.id == id }
            prefs[AGENTS_KEY] = json.encodeToString(agents)

            // Clear current agent if it was the deleted one
            if (prefs[CURRENT_AGENT_ID_KEY] == id) {
                prefs.remove(CURRENT_AGENT_ID_KEY)
            }
        }
    }

    /** Switch the active agent, apply its config to UserDefaults. */
    suspend fun switchToAgent(agent: AgentConfiguration) {
        // Update last-used timestamp
        saveAgent(agent)

        // Set as current
        context.agentStore.edit { prefs ->
            prefs[CURRENT_AGENT_ID_KEY] = agent.id
        }

        // Apply to UserDefaults (the settings that BaymaxApiService reads)
        with(com.simtropolis.baymax.BaymaxApplication.instance.settingsRepository) {
            saveSettings(agent.url, agent.secret)
        }
    }

    /** Ensure the current UserDefaults config is saved as an agent. */
    suspend fun ensureCurrentAgentInList() {
        val repo = com.simtropolis.baymax.BaymaxApplication.instance.settingsRepository
        val currentUrl = repo.baseUrl
        val currentSecret = repo.secretKey
        if (currentUrl.isEmpty() || currentSecret.isEmpty()) return

        val existing = savedAgents.firstOrNull { it.url == currentUrl && it.secret == currentSecret }
        if (existing != null) {
            // Already saved — just ensure it's the current one
            context.agentStore.edit { prefs ->
                prefs[CURRENT_AGENT_ID_KEY] = existing.id
            }
        } else {
            // Create a new agent with a default name based on URL pattern
            val defaultName = AgentConfiguration.defaultNameFor(currentUrl)
            val agent = AgentConfiguration(name = defaultName, url = currentUrl, secret = currentSecret)
            saveAgent(agent)
            context.agentStore.edit { prefs ->
                prefs[CURRENT_AGENT_ID_KEY] = agent.id
            }
        }
    }

    /** Reset to trial mode — clear saved agents and reset settings. */
    suspend fun resetToTrial() {
        context.agentStore.edit { prefs ->
            prefs.clear()
        }
    }
}
