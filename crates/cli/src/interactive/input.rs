use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Insert(char),
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveToStart,
    MoveToEnd,
    NewLine,
    PreviousHistory,
    NextHistory,
    Submit,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOutcome {
    Updated,
    Submitted(String),
    Canceled,
    Unchanged,
}

#[derive(Debug, Clone, Default)]
pub struct InputEditor {
    buffer: String,
    cursor_byte_offset: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    draft_before_history: Option<String>,
}

impl InputEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn cursor_byte_offset(&self) -> usize {
        self.cursor_byte_offset
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn set_buffer(&mut self, buffer: impl Into<String>) {
        self.buffer = buffer.into();
        self.cursor_byte_offset = self.buffer.len();
        self.clear_history_navigation();
    }

    pub fn handle_event(&mut self, event: InputEvent) -> InputOutcome {
        match event {
            InputEvent::Insert(character) => {
                self.buffer.insert(self.cursor_byte_offset, character);
                self.cursor_byte_offset += character.len_utf8();
                self.clear_history_navigation();
                InputOutcome::Updated
            }
            InputEvent::Backspace => {
                let Some(range) = self.previous_character_range() else {
                    return InputOutcome::Unchanged;
                };
                self.buffer.replace_range(range.clone(), "");
                self.cursor_byte_offset = range.start;
                self.clear_history_navigation();
                InputOutcome::Updated
            }
            InputEvent::Delete => {
                let Some(range) = self.next_character_range() else {
                    return InputOutcome::Unchanged;
                };
                self.buffer.replace_range(range, "");
                self.clear_history_navigation();
                InputOutcome::Updated
            }
            InputEvent::MoveLeft => {
                let Some(range) = self.previous_character_range() else {
                    return InputOutcome::Unchanged;
                };
                self.cursor_byte_offset = range.start;
                InputOutcome::Updated
            }
            InputEvent::MoveRight => {
                let Some(range) = self.next_character_range() else {
                    return InputOutcome::Unchanged;
                };
                self.cursor_byte_offset = range.end;
                InputOutcome::Updated
            }
            InputEvent::MoveToStart => {
                if self.cursor_byte_offset == 0 {
                    return InputOutcome::Unchanged;
                }
                self.cursor_byte_offset = 0;
                InputOutcome::Updated
            }
            InputEvent::MoveToEnd => {
                if self.cursor_byte_offset == self.buffer.len() {
                    return InputOutcome::Unchanged;
                }
                self.cursor_byte_offset = self.buffer.len();
                InputOutcome::Updated
            }
            InputEvent::NewLine => {
                self.buffer.insert(self.cursor_byte_offset, '\n');
                self.cursor_byte_offset += 1;
                self.clear_history_navigation();
                InputOutcome::Updated
            }
            InputEvent::PreviousHistory => self.navigate_history_back(),
            InputEvent::NextHistory => self.navigate_history_forward(),
            InputEvent::Submit => {
                let submitted = self.buffer.trim_end().to_string();
                if submitted.trim().is_empty() {
                    return InputOutcome::Unchanged;
                }
                self.push_history(submitted.clone());
                self.buffer.clear();
                self.cursor_byte_offset = 0;
                self.clear_history_navigation();
                InputOutcome::Submitted(submitted)
            }
            InputEvent::Cancel => InputOutcome::Canceled,
        }
    }

    pub fn push_history(&mut self, entry: String) {
        if entry.trim().is_empty() || self.history.last() == Some(&entry) {
            return;
        }
        self.history.push(entry);
    }

    fn navigate_history_back(&mut self) -> InputOutcome {
        if self.history.is_empty() {
            return InputOutcome::Unchanged;
        }

        let next_index = match self.history_index {
            Some(0) => return InputOutcome::Unchanged,
            Some(index) => index - 1,
            None => {
                self.draft_before_history = Some(self.buffer.clone());
                self.history.len() - 1
            }
        };
        self.history_index = Some(next_index);
        self.buffer = self.history[next_index].clone();
        self.cursor_byte_offset = self.buffer.len();
        InputOutcome::Updated
    }

    fn navigate_history_forward(&mut self) -> InputOutcome {
        let Some(index) = self.history_index else {
            return InputOutcome::Unchanged;
        };

        if index + 1 < self.history.len() {
            let next_index = index + 1;
            self.history_index = Some(next_index);
            self.buffer = self.history[next_index].clone();
        } else {
            self.history_index = None;
            self.buffer = self.draft_before_history.take().unwrap_or_default();
        }
        self.cursor_byte_offset = self.buffer.len();
        InputOutcome::Updated
    }

    fn clear_history_navigation(&mut self) {
        self.history_index = None;
        self.draft_before_history = None;
    }

    fn previous_character_range(&self) -> Option<Range<usize>> {
        if self.cursor_byte_offset == 0 {
            return None;
        }
        let start = self.buffer[..self.cursor_byte_offset]
            .char_indices()
            .next_back()
            .map(|(byte_offset, _)| byte_offset)?;
        Some(start..self.cursor_byte_offset)
    }

    fn next_character_range(&self) -> Option<Range<usize>> {
        if self.cursor_byte_offset == self.buffer.len() {
            return None;
        }

        let mut characters = self.buffer[self.cursor_byte_offset..].char_indices();
        let (_, character) = characters.next()?;
        Some(self.cursor_byte_offset..self.cursor_byte_offset + character.len_utf8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_multiline_unicode_input() {
        let mut editor = InputEditor::new();

        assert_eq!(
            editor.handle_event(InputEvent::Insert('h')),
            InputOutcome::Updated
        );
        assert_eq!(
            editor.handle_event(InputEvent::Insert('é')),
            InputOutcome::Updated
        );
        assert_eq!(
            editor.handle_event(InputEvent::NewLine),
            InputOutcome::Updated
        );
        assert_eq!(
            editor.handle_event(InputEvent::Insert('!')),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "hé\n!");
        assert_eq!(editor.cursor_byte_offset(), "hé\n!".len());

        assert_eq!(
            editor.handle_event(InputEvent::MoveLeft),
            InputOutcome::Updated
        );
        assert_eq!(
            editor.handle_event(InputEvent::Backspace),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "hé!");
    }

    #[test]
    fn submits_non_empty_input_and_records_history() {
        let mut editor = InputEditor::new();
        editor.set_buffer("hello\n");

        assert_eq!(
            editor.handle_event(InputEvent::Submit),
            InputOutcome::Submitted("hello".to_string())
        );
        assert_eq!(editor.buffer(), "");
        assert_eq!(editor.history(), &["hello".to_string()]);
    }

    #[test]
    fn navigates_history_and_restores_draft() {
        let mut editor = InputEditor::new();
        editor.push_history("first".to_string());
        editor.push_history("second".to_string());
        editor.set_buffer("draft");

        assert_eq!(
            editor.handle_event(InputEvent::PreviousHistory),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "second");
        assert_eq!(
            editor.handle_event(InputEvent::PreviousHistory),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "first");
        assert_eq!(
            editor.handle_event(InputEvent::NextHistory),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "second");
        assert_eq!(
            editor.handle_event(InputEvent::NextHistory),
            InputOutcome::Updated
        );
        assert_eq!(editor.buffer(), "draft");
    }
}
