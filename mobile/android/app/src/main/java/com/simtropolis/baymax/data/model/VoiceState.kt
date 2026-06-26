package com.simtropolis.baymax.data.model

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
    val isSpeaking: Boolean = false
)
