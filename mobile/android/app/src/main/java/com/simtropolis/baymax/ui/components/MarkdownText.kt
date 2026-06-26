package com.simtropolis.baymax.ui.components

import android.text.method.LinkMovementMethod
import android.widget.TextView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import io.noties.markwon.Markwon
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin
import io.noties.markwon.ext.tasklist.TaskListPlugin

// HtmlPlugin requires separate dependency — not included by default

/**
 * Renders markdown text with syntax highlighting, tables, lists, and code blocks.
 * Uses Markwon library for rendering.
 * Mirrors iOS MarkdownTableView.
 *
 * Usage:
 *   MarkdownText(
 *       text = "# Hello\nThis is **bold** and `code`",
 *       modifier = Modifier.padding(...)
 *   )
 */
@Suppress("UNUSED_PARAMETER")
@Composable
fun MarkdownText(
    text: String,
    modifier: Modifier = Modifier,
    textColor: Color = MaterialTheme.colorScheme.onSurface,
    codeBackgroundColor: Color = Color(0xFF1E1E1E),
    codeTextColor: Color = Color(0xFFD4D4D4)
) {
    if (text.isBlank()) return

    val markwon = remember {
        Markwon.builder(com.simtropolis.baymax.BaymaxApplication.instance)
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(com.simtropolis.baymax.BaymaxApplication.instance))
            .usePlugin(TaskListPlugin.create(com.simtropolis.baymax.BaymaxApplication.instance))
            // HtmlPlugin requires separate dependency
            .build()
    }

    val spannable = remember(text) {
        markwon.toMarkdown(text)
    }

    Column(modifier = modifier) {
        // Render inline markdown text
        AndroidView(
            factory = { context ->
                TextView(context).apply {
                    textSize = 16f
                    setTextColor(textColor.toArgb())
                    movementMethod = LinkMovementMethod.getInstance()
                }
            },
            update = { textView ->
                markwon.setParsedMarkdown(textView, spannable)
            }
        )
    }
}

/**
 * Simple code block renderer using monospace font and dark background.
 * For inline code use backticks within MarkdownText; for multi-line
 * code blocks, Markwon handles them natively.
 */
@Suppress("UNUSED_PARAMETER")
@Composable
fun CodeBlock(
    code: String,
    modifier: Modifier = Modifier,
    backgroundColor: Color = Color(0xFF1E1E1E),
    textColor: Color = Color(0xFFD4D4D4)
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .background(
                color = backgroundColor,
                shape = RoundedCornerShape(8.dp)
            )
            .padding(12.dp)
    ) {
        Text(
            text = code,
            color = textColor,
            fontFamily = FontFamily.Monospace,
            fontSize = 13.sp,
            lineHeight = 18.sp
        )
    }
}
