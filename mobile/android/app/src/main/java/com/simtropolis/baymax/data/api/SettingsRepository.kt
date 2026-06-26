package com.simtropolis.baymax.data.api

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.runBlocking

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "baymax_settings")

class SettingsRepository(private val context: Context) {
    
    companion object {
        private val BASE_URL_KEY = stringPreferencesKey("baymax_base_url")
        private val SECRET_KEY_KEY = stringPreferencesKey("baymax_secret_key")
        
        const val DEFAULT_BASE_URL = "https://demo-baymaxd.fly.dev"
        const val DEFAULT_SECRET_KEY = "test"
    }
    
    // Synchronous getters for immediate use
    val baseUrl: String
        get() = runBlocking {
            context.dataStore.data.first()[BASE_URL_KEY] ?: DEFAULT_BASE_URL
        }
    
    val secretKey: String
        get() = runBlocking {
            context.dataStore.data.first()[SECRET_KEY_KEY] ?: DEFAULT_SECRET_KEY
        }
    
    // Flow-based observers
    val baseUrlFlow: Flow<String> = context.dataStore.data.map { preferences ->
        preferences[BASE_URL_KEY] ?: DEFAULT_BASE_URL
    }
    
    val secretKeyFlow: Flow<String> = context.dataStore.data.map { preferences ->
        preferences[SECRET_KEY_KEY] ?: DEFAULT_SECRET_KEY
    }
    
    val isTrialModeFlow: Flow<Boolean> = baseUrlFlow.map { url ->
        url.contains("demo-baymaxd.fly.dev")
    }
    
    suspend fun saveSettings(baseUrl: String, secretKey: String) {
        context.dataStore.edit { preferences ->
            preferences[BASE_URL_KEY] = baseUrl
            preferences[SECRET_KEY_KEY] = secretKey
        }
    }
    
    suspend fun resetToTrialMode() {
        context.dataStore.edit { preferences ->
            preferences[BASE_URL_KEY] = DEFAULT_BASE_URL
            preferences[SECRET_KEY_KEY] = DEFAULT_SECRET_KEY
        }
    }
}
