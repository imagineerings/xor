package com.simtropolis.sim.data.model

import androidx.compose.ui.graphics.Color
import com.simtropolis.sim.R

/**
 * Voice input modes, matching iOS EnhancedVoiceManager / ContinuousVoiceManager.
 */
enum class VoiceMode {
    /** Tap-to-record, transcribe on stop, manual submit. */
    Normal,

    /** Tap-to-record, live partials, auto-submit on silence. */
    Transcribe,

    /** Always listening, live partials, TTS responses, hands-free conversation. */
    Continuous
}

/**
 * Observable voice state exposed by VoiceManager.
 */
data class VoiceState(
    val mode: VoiceMode = VoiceMode.Normal,
    val isListening: Boolean = false,
    val transcription: String = "",
    val isSpeaking: Boolean = false,
    val stateLabel: String = "Idle"
) {
    val iconRes: Int
        get() = when (stateLabel) {
            "Listening" -> android.R.drawable.ic_btn_speak_now
            "Processing" -> android.R.drawable.ic_popup_sync
            "Speaking" -> android.R.drawable.ic_lock_silent_mode_off
            "Error" -> android.R.drawable.stat_notify_error
            else -> R.drawable.ic_sim_outline
        }

    val tintColor: Color
        get() = when (stateLabel) {
            "Listening" -> Color(0xFF2196F3)
            "Processing" -> Color(0xFFFFC107)
            "Speaking" -> Color(0xFF4CAF50)
            "Error" -> Color(0xFFF44336)
            else -> Color(0xFF666666)
        }
}
