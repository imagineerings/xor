package com.simtropolis.baymax.ui.components

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle

object SyntaxHighlighter {
    private val supportedLanguages = setOf(
        "swift", "python", "javascript", "js", "typescript", "ts", "json",
        "shell", "sh", "bash", "ruby", "go", "rust", "rs", "sql", "html", "css"
    )

    private val stringColor = Color(0xFFFFA657)
    private val commentColor = Color(0xFF7EE787)
    private val keywordColor = Color(0xFF79C0FF)
    private val numberColor = Color(0xFF56D4DD)
    private val functionColor = Color(0xFFFFD866)
    private val typeColor = Color(0xFF56D4DD)

    fun supportedLanguages(): List<String> = supportedLanguages.sorted()

    fun highlight(code: String, language: String): AnnotatedString {
        val normalizedLanguage = language.lowercase()
        if (normalizedLanguage !in supportedLanguages) return AnnotatedString(code)

        val patterns = listOf(
            Regex("""//.*|#.*|/\*[\s\S]*?\*/""") to SpanStyle(commentColor),
            Regex(""""(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'""") to SpanStyle(stringColor),
            Regex("""\b\d+(?:\.\d+)?\b""") to SpanStyle(numberColor),
            Regex("""\b[A-Z][A-Za-z0-9_]*\b""") to SpanStyle(typeColor),
            Regex("""\b[A-Za-z_][A-Za-z0-9_]*(?=\s*\()""") to SpanStyle(functionColor),
            keywordRegex(normalizedLanguage) to SpanStyle(keywordColor)
        )

        val spans = mutableListOf<Triple<Int, Int, SpanStyle>>()
        for ((regex, style) in patterns) {
            regex.findAll(code).forEach { match ->
                if (spans.none { match.range.first < it.second && match.range.last + 1 > it.first }) {
                    spans.add(Triple(match.range.first, match.range.last + 1, style))
                }
            }
        }

        return buildAnnotatedString {
            var cursor = 0
            for ((start, end, style) in spans.sortedBy { it.first }) {
                if (cursor < start) append(code.substring(cursor, start))
                withStyle(style) { append(code.substring(start, end)) }
                cursor = end
            }
            if (cursor < code.length) append(code.substring(cursor))
        }
    }

    private fun keywordRegex(language: String): Regex {
        val common = "if|else|for|while|return|class|struct|enum|fun|func|let|var|const|import|from|as|try|catch|throw|throws|async|await|true|false|null|nil"
        val extra = when (language) {
            "python" -> "|def|elif|with|lambda|None|self"
            "rust", "rs" -> "|fn|impl|trait|match|pub|mut|crate|mod|use|Some|None|Ok|Err"
            "go" -> "|func|package|defer|go|chan|interface|map|range"
            "sql" -> "|SELECT|FROM|WHERE|JOIN|LEFT|RIGHT|INSERT|UPDATE|DELETE|CREATE|TABLE|GROUP|ORDER|BY|LIMIT"
            "html" -> "|DOCTYPE|html|head|body|div|span|script|style"
            else -> ""
        }
        return Regex("""\b($common$extra)\b""", RegexOption.IGNORE_CASE)
    }
}
