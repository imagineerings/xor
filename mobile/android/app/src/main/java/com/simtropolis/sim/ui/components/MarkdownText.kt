package com.simtropolis.sim.ui.components

import android.text.method.LinkMovementMethod
import android.widget.TextView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
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
        Markwon.builder(com.simtropolis.sim.SimApplication.instance)
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(com.simtropolis.sim.SimApplication.instance))
            .usePlugin(TaskListPlugin.create(com.simtropolis.sim.SimApplication.instance))
            // HtmlPlugin requires separate dependency
            .build()
    }

    val segments = remember(text) { splitMarkdownCodeBlocks(text) }

    Column(modifier = modifier) {
        for (segment in segments) {
            when (segment) {
                is MarkdownSegment.TextSegment -> {
                    val spannable = remember(segment.text) {
                        markwon.toMarkdown(segment.text)
                    }
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

                is MarkdownSegment.CodeSegment -> {
                    CodeBlock(
                        code = segment.code,
                        language = segment.language,
                        backgroundColor = codeBackgroundColor,
                        textColor = codeTextColor,
                        modifier = Modifier.padding(vertical = 6.dp)
                    )
                }
            }
        }
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
    language: String = "",
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
        Column {
            if (language.isNotBlank()) {
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    Surface(
                        color = Color.White.copy(alpha = 0.12f),
                        shape = RoundedCornerShape(4.dp)
                    ) {
                        Text(
                            text = language.uppercase(),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                            color = Color.White.copy(alpha = 0.86f),
                            fontSize = 10.sp,
                            fontWeight = FontWeight.SemiBold
                        )
                    }
                }
                Spacer(modifier = Modifier.height(6.dp))
            }
            Text(
                text = SyntaxHighlighter.highlight(code, language),
                color = textColor,
                fontFamily = FontFamily.Monospace,
                fontSize = 13.sp,
                lineHeight = 18.sp
            )
        }
    }
}

private sealed class MarkdownSegment {
    data class TextSegment(val text: String) : MarkdownSegment()
    data class CodeSegment(val language: String, val code: String) : MarkdownSegment()
}

private fun splitMarkdownCodeBlocks(markdown: String): List<MarkdownSegment> {
    val regex = Regex("""```([A-Za-z0-9_+\-.]*)\n([\s\S]*?)```""")
    val segments = mutableListOf<MarkdownSegment>()
    var cursor = 0

    for (match in regex.findAll(markdown)) {
        if (match.range.first > cursor) {
            val text = markdown.substring(cursor, match.range.first)
            if (text.isNotBlank()) segments.add(MarkdownSegment.TextSegment(text))
        }
        segments.add(
            MarkdownSegment.CodeSegment(
                language = match.groupValues[1],
                code = match.groupValues[2].trimEnd()
            )
        )
        cursor = match.range.last + 1
    }

    if (cursor < markdown.length) {
        val text = markdown.substring(cursor)
        if (text.isNotBlank()) segments.add(MarkdownSegment.TextSegment(text))
    }

    return segments.ifEmpty { listOf(MarkdownSegment.TextSegment(markdown)) }
}
