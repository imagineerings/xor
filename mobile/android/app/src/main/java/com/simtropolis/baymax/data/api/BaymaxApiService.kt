package com.simtropolis.baymax.data.api

import android.util.Log
import com.simtropolis.baymax.data.model.*
import com.simtropolis.baymax.util.NoticeManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

sealed class ApiResult<out T> {
    data class Success<T>(val data: T) : ApiResult<T>()
    data class Error(val message: String, val code: Int? = null) : ApiResult<Nothing>()
}

class BaymaxApiService(
    private val settingsRepository: SettingsRepository
) {
    private val TAG = "BaymaxApiService"

    val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
        explicitNulls = false  // Don't serialize null values - server expects them omitted
    }

    private val client = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    val isTrialMode: Boolean
        get() = settingsRepository.baseUrl.contains("demo-baymaxd.fly.dev")

    private val baseUrl: String
        get() = settingsRepository.baseUrl

    private val secretKey: String
        get() = settingsRepository.secretKey

    private fun createRequest(path: String): Request.Builder {
        return Request.Builder()
            .url("${baseUrl}$path")
            .header("X-Secret-Key", secretKey)
    }

    // Test connection
    suspend fun testConnection(): ApiResult<Boolean> = withContext(Dispatchers.IO) {
        try {
            val request = createRequest("/status").get().build()
            val response = client.newCall(request).execute()

            if (response.isSuccessful) {
                ApiResult.Success(true)
            } else {
                handleHTTPStatus(response.code)
                ApiResult.Error("HTTP ${response.code}: ${response.message}", response.code)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Connection test failed", e)
            handleAPIError(e)
            ApiResult.Error(e.message ?: "Connection failed")
        }
    }

    // Fetch sessions
    suspend fun fetchSessions(): ApiResult<List<ChatSession>> = withContext(Dispatchers.IO) {
        if (isTrialMode) {
            return@withContext ApiResult.Success(emptyList())
        }

        try {
            val request = createRequest("/sessions").get().build()
            val response = client.newCall(request).execute()

            if (response.isSuccessful) {
                val body = response.body?.string() ?: return@withContext ApiResult.Error("Empty response")
                val sessionsResponse = json.decodeFromString<SessionsResponse>(body)
                ApiResult.Success(sessionsResponse.sessions)
            } else {
                handleHTTPStatus(response.code)
                ApiResult.Error("HTTP ${response.code}", response.code)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to fetch sessions", e)
            handleAPIError(e)
            ApiResult.Error(e.message ?: "Failed to fetch sessions")
        }
    }

    // Start new agent session
    suspend fun startAgent(workingDir: String = "."): ApiResult<AgentResponse> = withContext(Dispatchers.IO) {
        try {
            val bodyJson = """{"working_dir": "$workingDir"}"""
            val requestBody = bodyJson.toRequestBody("application/json".toMediaType())

            val request = createRequest("/agent/start")
                .post(requestBody)
                .build()

            Log.d(TAG, "Starting agent with working_dir: $workingDir")
            val response = client.newCall(request).execute()

            if (response.isSuccessful) {
                val body = response.body?.string() ?: return@withContext ApiResult.Error("Empty response")
                Log.d(TAG, "Start agent response: $body")
                val agentResponse = json.decodeFromString<AgentResponse>(body)
                ApiResult.Success(agentResponse)
            } else {
                val errorBody = response.body?.string() ?: "No error details"
                Log.e(TAG, "Start agent failed: HTTP ${response.code}: $errorBody")
                handleHTTPStatus(response.code)
                ApiResult.Error("HTTP ${response.code}: $errorBody", response.code)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start agent", e)
            handleAPIError(e)
            ApiResult.Error(e.message ?: "Failed to start agent")
        }
    }

    // Resume agent session - used to activate model and extensions
    suspend fun resumeAgent(
        sessionId: String,
        loadModelAndExtensions: Boolean = false
    ): ApiResult<SessionResponse> = withContext(Dispatchers.IO) {
        try {
            if (!loadModelAndExtensions) {
                // Just fetch the session data without activating
                val request = createRequest("/sessions/$sessionId").get().build()
                val response = client.newCall(request).execute()

                if (response.isSuccessful) {
                    val body = response.body?.string() ?: return@withContext ApiResult.Error("Empty response")
                    val sessionResponse = json.decodeFromString<SessionResponse>(body)
                    ApiResult.Success(sessionResponse)
                } else {
                    val errorBody = response.body?.string() ?: "No error details"
                    handleHTTPStatus(response.code)
                    ApiResult.Error("HTTP ${response.code}: $errorBody", response.code)
                }
            } else {
                // Use /agent/resume to load model and extensions
                val bodyJson = """{"session_id": "$sessionId", "load_model_and_extensions": true}"""
                val requestBody = bodyJson.toRequestBody("application/json".toMediaType())

                val request = createRequest("/agent/resume")
                    .post(requestBody)
                    .build()

                Log.d(TAG, "Resuming agent with loadModelAndExtensions=true")
                val response = client.newCall(request).execute()

                if (response.isSuccessful) {
                    val body = response.body?.string() ?: return@withContext ApiResult.Error("Empty response")
                    Log.d(TAG, "Resume agent response: $body")
                    val sessionResponse = json.decodeFromString<SessionResponse>(body)
                    ApiResult.Success(sessionResponse)
                } else {
                    val errorBody = response.body?.string() ?: "No error details"
                    Log.e(TAG, "Resume agent failed: HTTP ${response.code}: $errorBody")
                    handleHTTPStatus(response.code)
                    ApiResult.Error("HTTP ${response.code}: $errorBody", response.code)
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to resume agent", e)
            handleAPIError(e)
            ApiResult.Error(e.message ?: "Failed to resume agent")
        }
    }

    // Update from session - applies system prompt and recipe
    suspend fun updateFromSession(sessionId: String): ApiResult<Unit> = withContext(Dispatchers.IO) {
        try {
            val bodyJson = """{"session_id": "$sessionId"}"""
            val requestBody = bodyJson.toRequestBody("application/json".toMediaType())

            val request = createRequest("/agent/update_from_session")
                .post(requestBody)
                .build()

            Log.d(TAG, "Updating from session: $sessionId")
            val response = client.newCall(request).execute()

            if (response.isSuccessful) {
                Log.d(TAG, "Update from session successful")
                ApiResult.Success(Unit)
            } else {
                val errorBody = response.body?.string() ?: "No error details"
                Log.e(TAG, "Update from session failed: HTTP ${response.code}: $errorBody")
                handleHTTPStatus(response.code)
                ApiResult.Error("HTTP ${response.code}: $errorBody", response.code)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to update from session", e)
            handleAPIError(e)
            ApiResult.Error(e.message ?: "Failed to update from session")
        }
    }

    // Stream chat with SSE
    fun streamChat(
        messages: List<Message>,
        sessionId: String
    ): Flow<SSEEvent> = flow {
        val chatRequest = ChatRequest(
            messages = messages,
            sessionId = sessionId
        )

        val requestBody = json.encodeToString(ChatRequest.serializer(), chatRequest)
        Log.d(TAG, "Chat request body: $requestBody")

        val request = Request.Builder()
            .url("${baseUrl}/reply")
            .header("X-Secret-Key", secretKey)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .post(requestBody.toRequestBody("application/json".toMediaType()))
            .build()

        Log.d(TAG, "Starting SSE stream for session: $sessionId")

        val sseClient = OkHttpClient.Builder()
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(0, TimeUnit.SECONDS) // No timeout for SSE
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()

        val response = sseClient.newCall(request).execute()

        if (!response.isSuccessful) {
            val errorBody = response.body?.string() ?: "Unknown error"
            Log.e(TAG, "SSE request failed: HTTP ${response.code}: $errorBody")
            throw IOException("HTTP ${response.code}: $errorBody")
        }

        val source = response.body?.source() ?: throw IOException("Empty response body")

        Log.d(TAG, "SSE connection established, reading events...")

        while (!source.exhausted()) {
            val line = source.readUtf8Line() ?: break

            if (line.startsWith("data: ")) {
                val eventData = line.removePrefix("data: ")
                if (eventData.isNotEmpty()) {
                    try {
                        Log.d(TAG, "SSE event data: $eventData")
                        val event = parseSSEEvent(eventData)
                        if (event != null) {
                            emit(event)

                            if (event is SSEEvent.FinishEvent) {
                                Log.d(TAG, "Stream finished: ${event.reason}")
                                break
                            }
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Failed to parse SSE event: $eventData", e)
                    }
                }
            }
        }

        response.close()
        Log.d(TAG, "SSE stream closed")
    }.flowOn(Dispatchers.IO)

    // ------------------------------------------------------------------
    // Notice Triggers
    // ------------------------------------------------------------------

    /**
     * Trigger notices based on HTTP status codes.
     * Mirrors iOS handleHTTPStatus() logic in AppNoticeCenter.
     */
    private fun handleHTTPStatus(code: Int) {
        if (isTrialMode) return // Suppress notices in trial mode

        when (code) {
            503 -> {
                Log.w(TAG, "HTTP 503 — tunnel disabled")
                NoticeManager.showTunnelDisabled()
            }

            502, 504 -> {
                Log.w(TAG, "HTTP $code — gateway error, tunnel may be down")
                NoticeManager.showTunnelUnreachable()
            }
        }
    }

    /**
     * Trigger notices based on exception type/content.
     * Mirrors iOS handleAPIError() logic.
     */
    private fun handleAPIError(error: Exception) {
        if (isTrialMode) return // Suppress notices in trial mode

        val message = error.message ?: ""

        when {
            message.contains("Unable to resolve host") ||
                    message.contains("Failed to connect") ||
                    message.contains("Network is unreachable") -> {
                // Connection failure — could be tunnel issue if URL is private
                val url = baseUrl
                if (url.startsWith("https://100.") ||
                    url.startsWith("http://100.") ||
                    url.contains(".ts.net") ||
                    url.contains("localhost") ||
                    url.contains("127.0.0.1")
                ) {
                    NoticeManager.showTunnelUnreachable()
                }
            }

            message.contains("json") ||
                    message.contains("decode") ||
                    message.contains("serialize") -> {
                // Decoding failure — app may need update
                NoticeManager.showAppNeedsUpdate()
            }
        }
    }

    private fun parseSSEEvent(data: String): SSEEvent? {
        return try {
            val typeRegex = """"type"\s*:\s*"([^"]+)"""".toRegex()
            val typeMatch = typeRegex.find(data)
            val type = typeMatch?.groupValues?.get(1)

            Log.d(TAG, "Parsing SSE event type: $type")

            when (type) {
                "Message" -> json.decodeFromString<SSEEvent.MessageEvent>(data)
                "Error" -> json.decodeFromString<SSEEvent.ErrorEvent>(data)
                "Finish" -> json.decodeFromString<SSEEvent.FinishEvent>(data)
                "ModelChange" -> json.decodeFromString<SSEEvent.ModelChangeEvent>(data)
                "Ping" -> json.decodeFromString<SSEEvent.PingEvent>(data)
                "UpdateConversation" -> json.decodeFromString<SSEEvent.UpdateConversationEvent>(data)
                else -> {
                    Log.w(TAG, "Unknown SSE event type: $type")
                    null
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "SSE parse error for: $data", e)
            null
        }
    }
}
