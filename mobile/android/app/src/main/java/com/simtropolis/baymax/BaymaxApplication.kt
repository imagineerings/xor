package com.simtropolis.baymax

import android.app.Application
import com.simtropolis.baymax.data.api.AgentRepository
import com.simtropolis.baymax.data.api.BaymaxApiService
import com.simtropolis.baymax.data.api.SettingsRepository
import com.simtropolis.baymax.data.repository.FavoriteSessionsStorage
import com.simtropolis.baymax.data.repository.ThemeManager
import com.simtropolis.baymax.data.repository.TrialModeManager

class BaymaxApplication : Application() {

    lateinit var settingsRepository: SettingsRepository
        private set

    lateinit var apiService: BaymaxApiService
        private set

    lateinit var agentRepository: AgentRepository
        private set

    lateinit var trialModeManager: TrialModeManager
        private set

    lateinit var favoriteSessionsStorage: FavoriteSessionsStorage
        private set

    override fun onCreate() {
        super.onCreate()
        instance = this

        // Initialize repositories and services
        settingsRepository = SettingsRepository(this)
        apiService = BaymaxApiService(settingsRepository)
        agentRepository = AgentRepository(this)
        trialModeManager = TrialModeManager(this)
        favoriteSessionsStorage = FavoriteSessionsStorage(this)
        ThemeManager.initialize(this)
    }

    companion object {
        lateinit var instance: BaymaxApplication
            private set
    }
}
