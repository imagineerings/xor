package com.simtropolis.baymax.data.repository

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import com.simtropolis.baymax.data.model.VoiceMode
import com.simtropolis.baymax.data.model.VoiceState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Manages Android SpeechRecognizer and TextToSpeech for voice interaction.
 * Mirrors iOS EnhancedVoiceManager + ContinuousVoiceManager.
 *
 * Usage:
 *   val voiceManager = VoiceManager(context)
 *   voiceManager.callback = object : VoiceManagerCallback { ... }
 *   voiceManager.startListening()
 */
class VoiceManager(private val context: Context) {

    private var speechRecognizer: SpeechRecognizer? = null
    private var recognitionListener: RecognitionListener? = null

    /** Consumer callbacks — set by ChatViewModel. */
    var callback: VoiceManagerCallback? = null

    private val _state = MutableStateFlow(VoiceState())
    val state: StateFlow<VoiceState> = _state.asStateFlow()

    /** Current voice mode. Switching modes may stop ongoing recognition. */
    var mode: VoiceMode
        get() = _state.value.mode
        set(value) {
            if (value != _state.value.mode && _state.value.isListening) {
                stopListening()
            }
            _state.value = _state.value.copy(mode = value)
        }

    // ------------------------------------------------------------------
    // Speech Recognition
    // ------------------------------------------------------------------

    /** Start listening (microphone). Requires RECORD_AUDIO permission. */
    fun startListening() {
        if (_state.value.isListening) return

        val recognizer = SpeechRecognizer.createSpeechRecognizer(context)
            ?: return // device doesn't support speech recognition

        val listener = createRecognitionListener()
        recognizer.setRecognitionListener(listener)

        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(
                RecognizerIntent.EXTRA_LANGUAGE_MODEL,
                RecognizerIntent.LANGUAGE_MODEL_FREE_FORM
            )
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
        }

        recognizer.startListening(intent)

        speechRecognizer = recognizer
        recognitionListener = listener
        _state.value = _state.value.copy(isListening = true)
    }

    /** Stop listening and finalize any pending transcription. */
    fun stopListening() {
        speechRecognizer?.apply {
            stopListening()
            destroy()
        }
        speechRecognizer = null
        recognitionListener = null
        _state.value = _state.value.copy(isListening = false)
    }

    /** Cancel listening without producing a final result. */
    fun cancelListening() {
        speechRecognizer?.apply {
            cancel()
            destroy()
        }
        speechRecognizer = null
        recognitionListener = null
        _state.value = _state.value.copy(
            isListening = false,
            transcription = ""
        )
    }

    private fun createRecognitionListener(): RecognitionListener {
        return object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {}
            override fun onBeginningOfSpeech() {}
            override fun onRmsChanged(rmsdB: Float) {}
            override fun onBufferReceived(buffer: ByteArray?) {}
            override fun onEvent(eventType: Int, params: Bundle?) {}
            override fun onEndOfSpeech() {}

            override fun onPartialResults(partialResults: Bundle?) {
                val matches = partialResults
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                val text = matches?.firstOrNull() ?: ""
                _state.value = _state.value.copy(transcription = text)
                callback?.onTranscriptionUpdate(text)
            }

            override fun onResults(results: Bundle?) {
                val matches = results
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                val text = matches?.firstOrNull() ?: ""
                _state.value = _state.value.copy(
                    isListening = false,
                    transcription = text
                )
                callback?.onSubmitMessage(text)
            }

            override fun onError(error: Int) {
                _state.value = _state.value.copy(
                    isListening = false,
                    transcription = ""
                )
                // Don't propagate speech-recognition errors to the UI
                // as they're frequently transient (e.g. "No match")
            }
        }
    }

    // ------------------------------------------------------------------
    // Text-To-Speech
    // ------------------------------------------------------------------

    private var tts: android.speech.tts.TextToSpeech? = null
    private var ttsInitialized = false

    /** Speak a response text aloud. Initialises TTS lazily. */
    fun speakResponse(text: String) {
        if (!ttsInitialized) {
            tts = android.speech.tts.TextToSpeech(context) { status ->
                if (status == android.speech.tts.TextToSpeech.SUCCESS) {
                    ttsInitialized = true
                    _state.value = _state.value.copy(isSpeaking = true)
                    tts?.speak(text, android.speech.tts.TextToSpeech.QUEUE_FLUSH, null, null)
                }
            }
        } else {
            _state.value = _state.value.copy(isSpeaking = true)
            tts?.speak(text, android.speech.tts.TextToSpeech.QUEUE_FLUSH, null, null)
        }
    }

    /** Stop any ongoing TTS. */
    fun stopSpeaking() {
        tts?.stop()
        _state.value = _state.value.copy(isSpeaking = false)
    }

    /** Release all resources. Call from ViewModel.onCleared(). */
    fun release() {
        stopListening()
        stopSpeaking()
        tts?.shutdown()
        tts = null
        ttsInitialized = false
    }
}
