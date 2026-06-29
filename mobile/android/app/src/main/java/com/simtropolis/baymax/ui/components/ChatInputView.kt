package com.simtropolis.baymax.ui.components

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.simtropolis.baymax.data.model.VoiceMode
import com.simtropolis.baymax.data.model.VoiceState

@Composable
fun ChatInputView(
    text: String = "",
    onTextChange: ((String) -> Unit)? = null,
    onSubmit: (() -> Unit)? = null,
    // New parameter names for compatibility
    value: String = text,
    onValueChange: ((String) -> Unit)? = onTextChange,
    onSend: (() -> Unit)? = onSubmit,
    onStop: (() -> Unit)? = null,
    isLoading: Boolean = false,
    showPlusButton: Boolean = false,
    placeholder: String = "I want to...",
    // Voice parameters
    isListening: Boolean = false,
    voiceMode: VoiceMode? = null,
    voiceState: VoiceState? = null,
    onVoiceToggle: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    // Use whichever params are provided
    val actualText = if (onValueChange != null) value else text
    val actualOnChange = onValueChange ?: onTextChange ?: {}
    val actualOnSubmit = onSend ?: onSubmit ?: {}

    val focusRequester = remember { FocusRequester() }
    val canSubmit = actualText.isNotBlank()

    Surface(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        shape = RoundedCornerShape(32.dp),
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp,
        shadowElevation = 8.dp
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = 12.dp, bottom = 12.dp, start = 16.dp, end = 12.dp)
        ) {
            // Text field
            BasicTextField(
                value = actualText,
                onValueChange = actualOnChange,
                modifier = Modifier
                    .fillMaxWidth()
                    .focusRequester(focusRequester)
                    .padding(vertical = 8.dp),
                textStyle = TextStyle(
                    fontSize = 16.sp,
                    color = MaterialTheme.colorScheme.onSurface
                ),
                cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                maxLines = 4,
                decorationBox = { innerTextField ->
                    Box {
                        if (actualText.isEmpty() && !isListening) {
                            Text(
                                text = placeholder,
                                style = TextStyle(
                                    fontSize = 16.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            )
                        } else if (actualText.isEmpty() && isListening) {
                            Text(
                                text = "Listening...",
                                style = TextStyle(
                                    fontSize = 16.sp,
                                    color = MaterialTheme.colorScheme.primary
                                )
                            )
                        }
                        innerTextField()
                    }
                }
            )

            // Buttons row
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Left side: Plus button OR Mic button (mutually exclusive)
                if (showPlusButton && !isListening) {
                    IconButton(
                        onClick = { /* File attachment */ },
                        modifier = Modifier
                            .size(32.dp)
                            .border(
                                width = 0.5.dp,
                                color = MaterialTheme.colorScheme.outline,
                                shape = CircleShape
                            )
                    ) {
                        Icon(
                            imageVector = Icons.Default.Add,
                            contentDescription = "Add attachment",
                            modifier = Modifier.size(16.dp),
                            tint = MaterialTheme.colorScheme.onSurface
                        )
                    }
                } else if (onVoiceToggle != null) {
                    // Microphone button with mode indicator
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        // Show voice mode label when active
                        if (isListening && voiceMode != null && voiceMode != VoiceMode.Normal) {
                            Text(
                                text = voiceState?.stateLabel?.takeIf { it != "Idle" } ?: when (voiceMode) {
                                    VoiceMode.Transcribe -> "Auto"
                                    VoiceMode.Continuous -> "∞"
                                    else -> ""
                                },
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(end = 4.dp)
                            )
                        }
                        IconButton(
                            onClick = onVoiceToggle,
                            modifier = Modifier
                                .size(32.dp)
                                .clip(CircleShape)
                                .background(
                                    if (isListening) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)
                                    else Color.Transparent
                                )
                        ) {
                            Icon(
                                painter = painterResource(voiceState?.iconRes ?: if (isListening) android.R.drawable.ic_btn_speak_now else com.simtropolis.baymax.R.drawable.ic_baymax_outline),
                                contentDescription = if (isListening) "Stop listening" else "Start voice input",
                                modifier = Modifier.size(20.dp),
                                tint = voiceState?.tintColor ?: if (isListening) MaterialTheme.colorScheme.primary
                                else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                } else {
                    Spacer(modifier = Modifier.width(32.dp))
                }

                Spacer(modifier = Modifier.weight(1f))

                // Send/Stop button
                IconButton(
                    onClick = {
                        if (isLoading && onStop != null) {
                            onStop()
                        } else if (canSubmit) {
                            actualOnSubmit()
                        }
                    },
                    enabled = isLoading || canSubmit,
                    modifier = Modifier
                        .size(32.dp)
                        .clip(CircleShape)
                        .background(
                            when {
                                isLoading -> MaterialTheme.colorScheme.error
                                canSubmit -> MaterialTheme.colorScheme.onSurface
                                else -> MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)
                            }
                        )
                ) {
                    Icon(
                        imageVector = if (isLoading) Icons.Default.Stop else Icons.Default.ArrowUpward,
                        contentDescription = if (isLoading) "Stop" else "Send",
                        modifier = Modifier.size(18.dp),
                        tint = if (isLoading) Color.White else MaterialTheme.colorScheme.surface
                    )
                }
            }
        }
    }
}
