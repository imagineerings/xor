package com.simtropolis.baymax.data.repository

import android.content.Context
import com.simtropolis.baymax.data.model.VoiceMode
import com.simtropolis.baymax.data.model.VoiceState
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Continuous voice mode: always listening, with TTS responses and automatic
 * re-listening after the assistant finishes speaking.
 *
 * Mirrors iOS ContinuousVoiceManager.
 */
class ContinuousVoiceManager(private val context: Context) {

    private val voiceManager = VoiceManager(context)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private val _state = MutableStateFlow(VoiceState(mode = VoiceMode.Continuous))
    val state: StateFlow<VoiceState> = _state.asStateFlow()

    var callback: VoiceManagerCallback? = null

    /** Whether continuous voice is currently active. */
    var isVoiceMode: Boolean = false
        private set

    /** Track if we're in the middle of a speech+listen cycle. */
    private var isProcessing: Boolean = false

    init {
        voiceManager.callback = object : VoiceManagerCallback {
            override fun onTranscriptionUpdate(partial: String) {
                _state.value = _state.value.copy(transcription = partial)
                callback?.onTranscriptionUpdate(partial)
            }

            override fun onSubmitMessage(text: String) {
                if (text.isNotBlank()) {
                    isProcessing = true
                    _state.value = _state.value.copy(transcription = text)
                    callback?.onSubmitMessage(text)
                }
            }

            override fun onCancelRequest() {
                callback?.onCancelRequest()
            }
        }
    }

    /** Start continuous voice mode. */
    fun start() {
        isVoiceMode = true
        isProcessing = false
        voiceManager.mode = VoiceMode.Continuous
        voiceManager.startListening()
        _state.value = _state.value.copy(isListening = true)
    }

    /** Stop continuous voice mode. */
    fun stop() {
        isVoiceMode = false
        isProcessing = false
        voiceManager.stopListening()
        voiceManager.stopSpeaking()
        _state.value = _state.value.copy(isListening = false, isSpeaking = false, transcription = "")
    }

    /** Speak a response, then resume listening. */
    fun speakResponse(text: String) {
        _state.value = _state.value.copy(isSpeaking = true)

        voiceManager.speakResponse(text)

        // After TTS finishes, resume listening
        scope.launch {
            delay(estimateSpeechDuration(text))
            _state.value = _state.value.copy(isSpeaking = false)

            if (isVoiceMode && !isProcessing) {
                voiceManager.startListening()
                _state.value = _state.value.copy(isListening = true)
            }
        }
    }

    /** Handle user interaction (tap) — stops speech and resumes listening. */
    fun handleUserInteraction() {
        voiceManager.stopSpeaking()
        _state.value = _state.value.copy(isSpeaking = false)

        if (isVoiceMode) {
            isProcessing = false
            voiceManager.cancelListening()
            voiceManager.startListening()
            _state.value = _state.value.copy(isListening = true)
        }
    }

    /** Mark that processing is complete and re-enable listening. */
    fun onProcessingComplete() {
        isProcessing = false
        if (isVoiceMode) {
            voiceManager.startListening()
            _state.value = _state.value.copy(isListening = true)
        }
    }

    /** Release resources. */
    fun release() {
        stop()
        voiceManager.release()
        scope.cancel()
    }

    /** Rough estimate: ~100ms per character. */
    private fun estimateSpeechDuration(text: String): Long =
        (text.length * 100L).coerceIn(1000L, 30_000L)
}
