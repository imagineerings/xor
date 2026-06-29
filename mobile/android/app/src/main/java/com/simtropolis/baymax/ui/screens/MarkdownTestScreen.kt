package com.simtropolis.baymax.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.simtropolis.baymax.ui.components.MarkdownText

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MarkdownTestScreen(onDismiss: () -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Markdown Test") },
                actions = {
                    IconButton(onClick = onDismiss) {
                        Icon(Icons.Default.Check, contentDescription = "Done")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(16.dp)
        ) {
            item {
                Text("Assistant Message", style = MaterialTheme.typography.titleMedium)
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier.padding(vertical = 8.dp)
                ) {
                    MarkdownText(sampleMarkdown, modifier = Modifier.padding(12.dp))
                }
            }
            item {
                Text("User Message", style = MaterialTheme.typography.titleMedium)
                Surface(
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(vertical = 8.dp)
                ) {
                    MarkdownText(
                        sampleMarkdown,
                        modifier = Modifier.padding(12.dp),
                        textColor = MaterialTheme.colorScheme.onPrimary
                    )
                }
            }
        }
    }
}

private val sampleMarkdown = """
# Markdown Coverage

This paragraph includes **bold**, _italic_, `inline code`, ~~strike~~, and [a link](https://simtropolis.com).

- Task lists
- Tables
- Fenced code blocks

| Feature | Status |
| --- | --- |
| Tables | Working |
| Code | Highlighted |

```swift
struct Message {
    let text: String
    func render() { print(text) }
}
```

```python
def greet(name):
    return f"hello {name}"
```

```typescript
const total: number = items.filter(item => item.active).length
```

```json
{"sessions": 5, "tokens": 450000000}
```

```rust
pub fn answer() -> Result<i32, Error> {
    Ok(42)
}
```

```sql
SELECT id, description FROM sessions WHERE total_tokens > 1000;
```
""".trimIndent()
