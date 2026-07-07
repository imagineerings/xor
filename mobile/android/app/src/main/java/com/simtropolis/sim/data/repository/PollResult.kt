package com.simtropolis.sim.data.repository

import com.simtropolis.sim.data.model.Message

/**
 * Result of a single polling cycle.
 * Mirrors the iOS polling logic in SessionPoller.
 */
sealed class PollResult {
    /** New or updated messages were detected. */
    data class Updated(val messages: List<Message>) : PollResult()

    /** No changes since the last poll. */
    data object NoChange : PollResult()

    /** An error occurred during polling. */
    data class Error(val exception: Exception) : PollResult()

    /** The session no longer exists on the server (HTTP 404). */
    data object SessionDeleted : PollResult()
}
