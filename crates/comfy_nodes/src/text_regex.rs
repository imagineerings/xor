use comfy_types::CancellationToken;
use fancy_regex::{Regex, RegexBuilder};
use thiserror::Error;

pub const NATIVE_TEXT_REGEX_MAX_PATTERN_BYTES: usize = 16 * 1024;
pub const NATIVE_TEXT_REGEX_MAX_INPUT_BYTES: usize = 1024 * 1024;
pub const NATIVE_TEXT_REGEX_MAX_MATCHES: usize = 100_000;
pub const NATIVE_TEXT_REGEX_MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;
pub const NATIVE_TEXT_REGEX_BACKTRACK_LIMIT: usize = 1_000_000;
const NATIVE_TEXT_REGEX_DELEGATE_SIZE_LIMIT: usize = 2 * 1024 * 1024;
const NATIVE_TEXT_REGEX_DELEGATE_DFA_SIZE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeTextRegexFlags {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTextRegexCaptureRows {
    capture_count: usize,
    rows: Vec<Vec<Option<String>>>,
    captured_bytes: usize,
}

impl NativeTextRegexCaptureRows {
    pub fn capture_count(&self) -> usize {
        self.capture_count
    }

    pub fn rows(&self) -> &[Vec<Option<String>>] {
        &self.rows
    }

    pub fn captured_bytes(&self) -> usize {
        self.captured_bytes
    }

    pub fn into_rows(self) -> Vec<Vec<Option<String>>> {
        self.rows
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NativeTextRegexError {
    #[error("regex pattern is {actual_bytes} bytes, above the {maximum_bytes}-byte limit")]
    PatternTooLarge {
        actual_bytes: usize,
        maximum_bytes: usize,
    },
    #[error("regex input is {actual_bytes} bytes, above the {maximum_bytes}-byte limit")]
    InputTooLarge {
        actual_bytes: usize,
        maximum_bytes: usize,
    },
    #[error("invalid regex pattern: {0}")]
    InvalidPattern(String),
    #[error("regex execution exceeded a bounded runtime limit: {0}")]
    ExecutionLimit(String),
    #[error("regex result exceeded the {maximum_matches}-match limit")]
    MatchLimit { maximum_matches: usize },
    #[error("regex captures exceeded the {maximum_bytes}-byte limit")]
    CaptureLimit { maximum_bytes: usize },
    #[error("regex execution was cancelled")]
    Cancelled,
}

#[derive(Clone, Copy)]
struct NativeTextRegexLimits {
    maximum_input_bytes: usize,
    maximum_matches: usize,
    maximum_capture_bytes: usize,
    backtrack_limit: usize,
}

impl Default for NativeTextRegexLimits {
    fn default() -> Self {
        Self {
            maximum_input_bytes: NATIVE_TEXT_REGEX_MAX_INPUT_BYTES,
            maximum_matches: NATIVE_TEXT_REGEX_MAX_MATCHES,
            maximum_capture_bytes: NATIVE_TEXT_REGEX_MAX_CAPTURE_BYTES,
            backtrack_limit: NATIVE_TEXT_REGEX_BACKTRACK_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct NativeTextRegex {
    regex: Regex,
    limits: NativeTextRegexLimits,
}

impl NativeTextRegex {
    pub fn checked(
        pattern: &str,
        flags: NativeTextRegexFlags,
    ) -> Result<Self, NativeTextRegexError> {
        Self::checked_with_limits(pattern, flags, NativeTextRegexLimits::default())
    }

    fn checked_with_limits(
        pattern: &str,
        flags: NativeTextRegexFlags,
        limits: NativeTextRegexLimits,
    ) -> Result<Self, NativeTextRegexError> {
        if pattern.len() > NATIVE_TEXT_REGEX_MAX_PATTERN_BYTES {
            return Err(NativeTextRegexError::PatternTooLarge {
                actual_bytes: pattern.len(),
                maximum_bytes: NATIVE_TEXT_REGEX_MAX_PATTERN_BYTES,
            });
        }
        let mut builder = RegexBuilder::new(pattern);
        builder
            .case_insensitive(flags.case_insensitive)
            .multi_line(flags.multi_line)
            .dot_matches_new_line(flags.dot_matches_new_line)
            .backtrack_limit(limits.backtrack_limit)
            .delegate_size_limit(NATIVE_TEXT_REGEX_DELEGATE_SIZE_LIMIT)
            .delegate_dfa_size_limit(NATIVE_TEXT_REGEX_DELEGATE_DFA_SIZE_LIMIT);
        let regex = builder
            .build()
            .map_err(|error| NativeTextRegexError::InvalidPattern(error.to_string()))?;
        Ok(Self { regex, limits })
    }

    pub fn capture_count(&self) -> usize {
        self.regex.captures_len()
    }

    pub fn is_match(
        &self,
        input: &str,
        cancellation: &CancellationToken,
    ) -> Result<bool, NativeTextRegexError> {
        self.validate_input(input, cancellation)?;
        let result = self
            .regex
            .is_match(input)
            .map_err(|error| NativeTextRegexError::ExecutionLimit(error.to_string()))?;
        self.check_cancellation(cancellation)?;
        Ok(result)
    }

    pub fn capture_rows(
        &self,
        input: &str,
        cancellation: &CancellationToken,
    ) -> Result<NativeTextRegexCaptureRows, NativeTextRegexError> {
        self.validate_input(input, cancellation)?;
        let capture_count = self.regex.captures_len();
        let mut rows = Vec::new();
        let mut captured_bytes = 0usize;
        for captures in self.regex.captures_iter(input) {
            self.check_cancellation(cancellation)?;
            if rows.len() >= self.limits.maximum_matches {
                return Err(NativeTextRegexError::MatchLimit {
                    maximum_matches: self.limits.maximum_matches,
                });
            }
            let captures = captures
                .map_err(|error| NativeTextRegexError::ExecutionLimit(error.to_string()))?;
            let mut row = Vec::with_capacity(capture_count);
            for index in 0..capture_count {
                let value = captures.get(index).map(|matched| matched.as_str());
                if let Some(value) = value {
                    captured_bytes = captured_bytes.checked_add(value.len()).ok_or(
                        NativeTextRegexError::CaptureLimit {
                            maximum_bytes: self.limits.maximum_capture_bytes,
                        },
                    )?;
                    if captured_bytes > self.limits.maximum_capture_bytes {
                        return Err(NativeTextRegexError::CaptureLimit {
                            maximum_bytes: self.limits.maximum_capture_bytes,
                        });
                    }
                }
                row.push(value.map(str::to_owned));
            }
            rows.push(row);
        }
        self.check_cancellation(cancellation)?;
        Ok(NativeTextRegexCaptureRows {
            capture_count,
            rows,
            captured_bytes,
        })
    }

    fn validate_input(
        &self,
        input: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeTextRegexError> {
        self.check_cancellation(cancellation)?;
        if input.len() > self.limits.maximum_input_bytes {
            return Err(NativeTextRegexError::InputTooLarge {
                actual_bytes: input.len(),
                maximum_bytes: self.limits.maximum_input_bytes,
            });
        }
        Ok(())
    }

    fn check_cancellation(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeTextRegexError> {
        cancellation
            .check()
            .map_err(|_| NativeTextRegexError::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn python_style_captures_lookaround_and_backreferences_are_preserved()
    -> Result<(), Box<dyn Error>> {
        let regex = NativeTextRegex::checked(
            r"(?P<word>\w+)\s+(?P=word)(?=!)",
            NativeTextRegexFlags::default(),
        )?;
        let captures = regex.capture_rows("go go! stop", &CancellationToken::default())?;
        assert_eq!(regex.capture_count(), 2);
        assert_eq!(captures.capture_count(), 2);
        assert_eq!(
            captures.rows(),
            &[vec![Some("go go".to_owned()), Some("go".to_owned())]]
        );
        Ok(())
    }

    #[test]
    fn flags_unicode_and_optional_capture_rows_are_exact() -> Result<(), Box<dyn Error>> {
        let regex = NativeTextRegex::checked(
            r"^(?<name>café).(x)?$",
            NativeTextRegexFlags {
                case_insensitive: true,
                multi_line: true,
                dot_matches_new_line: true,
            },
        )?;
        let captures = regex.capture_rows("CAFÉ\n\n", &CancellationToken::default())?;
        assert_eq!(
            captures.rows(),
            &[vec![
                Some("CAFÉ\n".to_owned()),
                Some("CAFÉ".to_owned()),
                None,
            ]]
        );
        Ok(())
    }

    #[test]
    fn invalid_patterns_and_all_resource_limits_fail_closed() -> Result<(), Box<dyn Error>> {
        assert!(matches!(
            NativeTextRegex::checked("(", NativeTextRegexFlags::default()),
            Err(NativeTextRegexError::InvalidPattern(_))
        ));
        assert!(matches!(
            NativeTextRegex::checked(
                &"a".repeat(NATIVE_TEXT_REGEX_MAX_PATTERN_BYTES + 1),
                NativeTextRegexFlags::default(),
            ),
            Err(NativeTextRegexError::PatternTooLarge { .. })
        ));

        let limits = NativeTextRegexLimits {
            maximum_input_bytes: 8,
            maximum_matches: 1,
            maximum_capture_bytes: 2,
            backtrack_limit: 8,
        };
        let input_limited =
            NativeTextRegex::checked_with_limits("a", NativeTextRegexFlags::default(), limits)?;
        assert_eq!(
            input_limited.is_match("123456789", &CancellationToken::default()),
            Err(NativeTextRegexError::InputTooLarge {
                actual_bytes: 9,
                maximum_bytes: 8,
            })
        );
        assert_eq!(
            input_limited.capture_rows("aa", &CancellationToken::default()),
            Err(NativeTextRegexError::MatchLimit { maximum_matches: 1 })
        );

        let capture_limited =
            NativeTextRegex::checked_with_limits("...", NativeTextRegexFlags::default(), limits)?;
        assert_eq!(
            capture_limited.capture_rows("abc", &CancellationToken::default()),
            Err(NativeTextRegexError::CaptureLimit { maximum_bytes: 2 })
        );

        let backtrack_limited = NativeTextRegex::checked_with_limits(
            r"(x+x+)+(?>y)",
            NativeTextRegexFlags::default(),
            NativeTextRegexLimits {
                maximum_input_bytes: 64,
                backtrack_limit: 1,
                ..limits
            },
        )?;
        assert!(matches!(
            backtrack_limited.is_match("xxxxxxxxxxy", &CancellationToken::default()),
            Err(NativeTextRegexError::ExecutionLimit(_))
        ));
        Ok(())
    }

    #[test]
    fn cancellation_is_checked_before_matching_and_between_capture_rows()
    -> Result<(), Box<dyn Error>> {
        let regex = NativeTextRegex::checked("a", NativeTextRegexFlags::default())?;
        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        assert_eq!(
            regex.is_match("a", &cancellation),
            Err(NativeTextRegexError::Cancelled)
        );
        assert_eq!(
            regex.capture_rows("aaa", &cancellation),
            Err(NativeTextRegexError::Cancelled)
        );
        Ok(())
    }
}
