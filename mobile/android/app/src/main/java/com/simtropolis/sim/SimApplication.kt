package com.simtropolis.sim

import android.app.Application
import com.simtropolis.sim.data.api.AgentRepository
import com.simtropolis.sim.data.api.SimApiService
import com.simtropolis.sim.data.api.SettingsRepository
import com.simtropolis.sim.data.repository.FavoriteSessionsStorage
import com.simtropolis.sim.data.repository.ThemeManager
import com.simtropolis.sim.data.repository.TrialModeManager

class SimApplication : Application() {

    lateinit var settingsRepository: SettingsRepository
        private set

    lateinit var apiService: SimApiService
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
        apiService = SimApiService(settingsRepository)
        agentRepository = AgentRepository(this)
        trialModeManager = TrialModeManager(this)
        favoriteSessionsStorage = FavoriteSessionsStorage(this)
        ThemeManager.initialize(this)
    }

    companion object {
        lateinit var instance: SimApplication
            private set
    }
}
