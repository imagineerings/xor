pub const MAX_AGENT_TEXT_RESPONSE_BYTES: usize = 512 * 1024;

pub const RESPONSE_TRUNCATED_NOTICE: &str =
    "\n\n[Response truncated because it exceeded the maximum assistant message size.]";

pub fn append_text_chunk(existing: &mut String, chunk: &str) -> Option<String> {
    if existing.ends_with(RESPONSE_TRUNCATED_NOTICE) {
        return None;
    }

    let remaining = MAX_AGENT_TEXT_RESPONSE_BYTES.saturating_sub(existing.len());
    if chunk.len() <= remaining {
        existing.push_str(chunk);
        return Some(chunk.to_string());
    }

    let mut emitted = String::new();
    if remaining > 0 {
        let truncate_ix = chunk
            .char_indices()
            .map(|(ix, _)| ix)
            .take_while(|ix| *ix <= remaining)
            .last()
            .unwrap_or(0);
        let fitting = &chunk[..truncate_ix];
        existing.push_str(fitting);
        emitted.push_str(fitting);
    }

    existing.push_str(RESPONSE_TRUNCATED_NOTICE);
    emitted.push_str(RESPONSE_TRUNCATED_NOTICE);
    Some(emitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_text_chunk_truncates_at_limit() {
        let mut existing = "a".repeat(MAX_AGENT_TEXT_RESPONSE_BYTES - 2);
        let emitted = append_text_chunk(&mut existing, "bcdef").unwrap();

        assert_eq!(emitted, format!("bc{RESPONSE_TRUNCATED_NOTICE}"));
        assert!(existing.ends_with(RESPONSE_TRUNCATED_NOTICE));
        assert_eq!(
            existing.trim_end_matches(RESPONSE_TRUNCATED_NOTICE).len(),
            MAX_AGENT_TEXT_RESPONSE_BYTES
        );
    }

    #[test]
    fn test_append_text_chunk_uses_utf8_boundary() {
        let mut existing = "a".repeat(MAX_AGENT_TEXT_RESPONSE_BYTES - 1);
        let emitted = append_text_chunk(&mut existing, "ébc").unwrap();

        assert_eq!(emitted, RESPONSE_TRUNCATED_NOTICE);
        assert!(existing.ends_with(RESPONSE_TRUNCATED_NOTICE));
    }

    #[test]
    fn test_append_text_chunk_ignores_chunks_after_truncation() {
        let mut existing = format!("text{RESPONSE_TRUNCATED_NOTICE}");

        assert_eq!(append_text_chunk(&mut existing, "later"), None);
        assert_eq!(existing, format!("text{RESPONSE_TRUNCATED_NOTICE}"));
    }
}
