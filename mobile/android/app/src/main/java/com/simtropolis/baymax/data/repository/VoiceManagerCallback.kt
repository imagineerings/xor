package com.simtropolis.baymax.data.repository

/**
 * Callbacks from VoiceManager to the consumer (ChatViewModel).
 * Mirrors the iOS voice manager callback pattern.
 */
interface VoiceManagerCallback {
    /** Called with partial transcription text while user is speaking (Transcribe / Continuous modes). */
    fun onTranscriptionUpdate(partial: String)

    /** Called with the final transcribed text when the utterance is ready to submit. */
    fun onSubmitMessage(text: String)

    /** Called when the user wants to cancel the current streaming request (voice shortcut). */
    fun onCancelRequest()
}
