use gateway::{PairingService, TelegramFormatter};

#[test]
fn test_telegram_formatter_end_to_end() {
    let markdown = r#"
Some **bold** and *italic* text.

```rust
fn main() {
    println!("Hello");
}
```

[Link](https://example.com) and `inline code` and ~~strike~~.
"#;

    let html = TelegramFormatter::format_to_html(markdown);
    assert!(html.contains("<b>bold</b>"));
    assert!(html.contains("<i>italic</i>"));
    assert!(html.contains("<pre><code class=\"language-rust\">"));
    assert!(html.contains("fn main()"));
    assert!(html.contains("<a href=\"https://example.com\">"));
    assert!(html.contains("Link"));
    assert!(html.contains("<code>inline code</code>"));
    assert!(html.contains("<s>strike</s>"));
}

#[test]
fn test_pairing_persistence() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_pairing_persistence.json");
    let _ = std::fs::remove_file(&path);

    {
        let mut service = PairingService::with_storage(&path);
        service.pair_platform_user("tg:123", "alice").unwrap();
        service.pair_platform_user("tg:456", "bob").unwrap();
        assert_eq!(service.count(), 2);
    }

    {
        let service = PairingService::with_storage(&path);
        assert_eq!(service.lookup_sim_user("tg:123"), Some("alice"));
        assert_eq!(service.lookup_sim_user("tg:456"), Some("bob"));
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_pairing_unlink() {
    let mut service = PairingService::new();
    service.pair_platform_user("tg:123", "alice").unwrap();
    assert!(service.is_paired("tg:123"));
    service.unlink("tg:123").unwrap();
    assert!(!service.is_paired("tg:123"));
    assert_eq!(service.count(), 0);
}

#[test]
fn test_telegram_formatter_split_long_message() {
    let text = "A".repeat(3000) + "\n\n" + &"B".repeat(2000);
    let chunks = TelegramFormatter::split_message(&text);
    assert!(chunks.len() >= 2);
    assert!(chunks[0].ends_with("\n\n"));
    for chunk in &chunks {
        assert!(chunk.len() <= 4096, "chunk too long: {}", chunk.len());
    }
}

#[test]
fn test_telegram_formatter_no_split_short_message() {
    let chunks = TelegramFormatter::split_message("Short message");
    assert_eq!(chunks.len(), 1);
}
