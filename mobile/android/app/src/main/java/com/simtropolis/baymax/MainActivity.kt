package com.simtropolis.baymax

import android.content.Intent
import android.content.pm.ApplicationInfo
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.simtropolis.baymax.ui.screens.ChatScreen
import com.simtropolis.baymax.ui.screens.HomeScreen
import com.simtropolis.baymax.ui.screens.MarkdownTestScreen
import com.simtropolis.baymax.ui.screens.SettingsScreen
import com.simtropolis.baymax.ui.screens.SplashScreen
import com.simtropolis.baymax.ui.screens.TaskDetailScreen
import com.simtropolis.baymax.ui.screens.TaskOutputDetailScreen
import com.simtropolis.baymax.ui.screens.ToolCallDetailScreen
import com.simtropolis.baymax.ui.screens.TrialModeInstructionsScreen
import com.simtropolis.baymax.ui.theme.BaymaxTheme
import com.simtropolis.baymax.ui.components.AppNoticeOverlay
import com.simtropolis.baymax.util.QRConfigHandler
import kotlinx.coroutines.delay
import java.net.URLDecoder
import java.net.URLEncoder

class MainActivity : ComponentActivity() {

    /** Track configuration status from deep link so we can show it in the UI. */
    private var _pendingConfigResult: ((Boolean, String?) -> Unit)? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            BaymaxTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    val splashActive = remember { mutableStateOf(true) }
                    Box(modifier = Modifier.fillMaxSize()) {
                        if (!splashActive.value) {
                            BaymaxNavigation()
                        }
                        AppNoticeOverlay()
                        if (splashActive.value) {
                            SplashScreen(splashActive)
                        }
                    }

                    LaunchedEffect(QRConfigHandler.configurationSuccess.value) {
                        if (QRConfigHandler.configurationSuccess.value) {
                            delay(3_000)
                            QRConfigHandler.configurationSuccess.value = false
                        }
                    }
                }
            }
        }

        // Handle the intent that launched the activity
        intent?.let { handleDeepLink(it) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleDeepLink(intent)
    }

    private fun handleDeepLink(intent: Intent) {
        val data = intent.data ?: return

        when (data.scheme) {
            "baymaxchat" -> {
                QRConfigHandler.handleDeepLink(data) { success, error ->
                    // Result will be shown via NoticeManager or config status view
                    if (success) {
                        println("✅ Configuration applied successfully via deep link")
                    } else {
                        println("❌ Configuration failed: $error")
                    }
                }
            }

            "baymax" -> {
                // Legacy baymax:// scheme — try to parse as configure
                QRConfigHandler.handleDeepLink(data) { success, error ->
                    if (success) {
                        println("✅ Configuration applied via legacy scheme")
                    } else {
                        println("❌ Legacy configuration failed: $error")
                    }
                }
            }

            else -> {
                println("⚠️ Unhandled URL scheme: ${data.scheme}")
            }
        }
    }
}

@Composable
fun BaymaxNavigation() {
    val navController = rememberNavController()

    NavHost(
        navController = navController,
        startDestination = "home"
    ) {
        composable("home") {
            HomeScreen(
                onNavigateToChat = { sessionId, initialMessage ->
                    val route = when {
                        sessionId != null -> "chat/$sessionId"
                        initialMessage != null -> {
                            val encoded = URLEncoder.encode(initialMessage, "UTF-8")
                            "chat/new?message=$encoded"
                        }

                        else -> "chat/new"
                    }
                    navController.navigate(route)
                },
                onNavigateToSettings = {
                    navController.navigate("settings")
                },
                onNavigateToTrialInstructions = {
                    navController.navigate("trial-instructions")
                }
            )
        }

        composable(
            route = "chat/{sessionId}?message={message}",
            arguments = listOf(
                navArgument("sessionId") {
                    type = NavType.StringType
                },
                navArgument("message") {
                    type = NavType.StringType
                    nullable = true
                    defaultValue = null
                }
            )
        ) { backStackEntry ->
            val sessionId = backStackEntry.arguments?.getString("sessionId")
            val encodedMessage = backStackEntry.arguments?.getString("message")
            val initialMessage = encodedMessage?.let {
                URLDecoder.decode(it, "UTF-8")
            }

            ChatScreen(
                sessionId = if (sessionId == "new") null else sessionId,
                initialMessage = initialMessage,
                onNavigateBack = {
                    navController.popBackStack()
                }
            )
        }

        composable("settings") {
            SettingsScreen(
                onNavigateBack = {
                    navController.popBackStack()
                },
                onNavigateToMarkdownTest = {
                    if (isDebugBuild()) {
                        navController.navigate("markdown-test")
                    }
                }
            )
        }

        if (isDebugBuild()) {
            composable("markdown-test") {
                MarkdownTestScreen(onDismiss = { navController.popBackStack() })
            }
        }

        composable("trial-instructions") {
            TrialModeInstructionsScreen(
                onDismiss = {
                    navController.popBackStack()
                },
                onNavigateToSettings = {
                    navController.popBackStack()
                    navController.navigate("settings")
                }
            )
        }

        composable("tool-call-detail") {
            ToolCallDetailScreen(
                onNavigateBack = {
                    navController.popBackStack()
                }
            )
        }

        composable("task-detail") {
            TaskDetailScreen(
                onNavigateBack = {
                    navController.popBackStack()
                },
                onNavigateToTaskOutput = {
                    navController.navigate("task-output-detail")
                }
            )
        }

        composable("task-output-detail") {
            TaskOutputDetailScreen(
                onNavigateBack = {
                    navController.popBackStack()
                }
            )
        }
    }
}

private fun isDebugBuild(): Boolean {
    return BaymaxApplication.instance.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
}
