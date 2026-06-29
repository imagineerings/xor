package com.simtropolis.baymax.data.repository

import android.content.Context
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

private val Context.favoriteDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "baymax_favorites"
)

/**
 * Manages favorite session IDs using DataStore.
 *
 * Mirrors iOS FavoriteSessionsStorage.swift.
 */
class FavoriteSessionsStorage(private val context: Context) {
    companion object {
        private const val TAG = "FavoriteSessionsStorage"
        private val FAVORITE_IDS_KEY = stringPreferencesKey("favorite_session_ids")
    }

    /** Flow of favorite session IDs, reactive for Compose UI. */
    val favoriteIdsFlow: Flow<Set<String>> = context.favoriteDataStore.data.map { preferences ->
        val raw = preferences[FAVORITE_IDS_KEY] ?: ""
        if (raw.isBlank()) {
            emptySet()
        } else {
            raw.split(",").filter { it.isNotBlank() }.toSet()
        }
    }

    /** Check if a session is favorited. */
    suspend fun isFavorite(sessionId: String): Boolean {
        return favoriteIdsFlow.first().contains(sessionId)
    }

    /** Toggle favorite status for a session. */
    suspend fun toggleFavorite(sessionId: String) {
        val current = favoriteIdsFlow.first()
        val updated = if (current.contains(sessionId)) {
            Log.d(TAG, "Removed favorite: $sessionId")
            current - sessionId
        } else {
            Log.d(TAG, "Added favorite: $sessionId")
            current + sessionId
        }
        saveFavorites(updated)
    }

    /** Add a session to favorites. */
    suspend fun addFavorite(sessionId: String) {
        val current = favoriteIdsFlow.first()
        if (!current.contains(sessionId)) {
            saveFavorites(current + sessionId)
        }
    }

    /** Remove a session from favorites. */
    suspend fun removeFavorite(sessionId: String) {
        val current = favoriteIdsFlow.first()
        if (current.contains(sessionId)) {
            saveFavorites(current - sessionId)
        }
    }

    private suspend fun saveFavorites(ids: Set<String>) {
        context.favoriteDataStore.edit { preferences ->
            preferences[FAVORITE_IDS_KEY] = ids.joinToString(",")
        }
        Log.d(TAG, "Saved ${ids.size} favorite sessions")
    }
}
