use serde::{Deserialize, Serialize};

use crate::{WorldActionControl, WorldControl};

// ---------------------------------------------------------------------------
// Semantic key groups
// ---------------------------------------------------------------------------

/// The semantic group a control key belongs to.
///
/// `Move` covers WASD movement keys; `Look` covers IJKL camera/look keys.
/// `Unknown` keys are tolerated (so teams can carry experiment bindings) but
/// are reported distinctly from validated groups so callers can warn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ControlKeyGroup {
    Move,
    Look,
    Unknown,
}

impl ControlKeyGroup {
    /// Classify a single (normalized, lower-case) key name into its group.
    pub fn classify(key: &str) -> Self {
        match key {
            "w" | "a" | "s" | "d" => Self::Move,
            "i" | "j" | "k" | "l" => Self::Look,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Look => "look",
            Self::Unknown => "unknown",
        }
    }
}

/// Mutually exclusive pairs of opposing direction keys within a group.
///
/// Semantically, an axis should not be pressed in both opposing directions
/// (with non-zero values) on the same frame; that is a wiring mistake, not a
/// strafing intent.
const MOVE_OPPOSING_PAIRS: &[(&str, &str)] = &[("w", "s"), ("a", "d")];
const LOOK_OPPOSING_PAIRS: &[(&str, &str)] = &[("i", "k"), ("j", "l")];

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// A parser that converts plain-text WASD/IJKL action strings into typed
/// `WorldControl` sequences.
///
/// The ported format is line-based, mirroring `projects/world-model`. Each
/// non-blank line encodes a frame: `<frame>:<key>=<value>[,<key>=<value>...]`.
/// `frame` is an unsigned, optionally zero-padded integer. Whitespace is
/// ignored around tokens. Lines beginning with `#` are comments.
///
/// Keys are matched case-insensitively and normalized to lower case. Values
/// must parse as `f32` in the range `[0.0, 1.0]` (inclusive of both bounds);
/// NaN is treated as malformed.
pub struct WorldActionControlParser;

#[derive(Clone, Debug)]
pub struct ControlParseError {
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for ControlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.column {
            Some(column) => write!(f, "line {}, column {}: {}", self.line, column, self.message),
            None => write!(f, "line {}: {}", self.line, self.message),
        }
    }
}

impl std::error::Error for ControlParseError {}

impl WorldActionControlParser {
    /// Parse an action string into a sequence of `WorldControl`, one per frame.
    pub fn parse(input: &str) -> Result<Vec<WorldControl>, Vec<ControlParseError>> {
        let mut errors: Vec<ControlParseError> = Vec::new();

        // (line_number, raw_token, parsed_value) for each successfully parsed
        // frame; storing the raw frame token lets us check padding width across
        // frames later.
        let mut frames: Vec<(usize, String, WorldControl)> = Vec::new();

        for (index, raw_line) in input.split('\n').enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match Self::parse_line(line, line_number) {
                Ok((raw_token, frame)) => {
                    frames.push((line_number, raw_token, frame));
                }
                Err(mut line_errors) => {
                    for err in line_errors.drain(..) {
                        errors.push(err);
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        if let Err(cross_errors) = validate_sequence(&frames) {
            for err in cross_errors {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(frames.into_iter().map(|(_, _, frame)| frame).collect())
        } else {
            Err(errors)
        }
    }

    fn parse_line(
        line: &str,
        line_number: usize,
    ) -> Result<(String, WorldControl), Vec<ControlParseError>> {
        let (frame_token, body) = match line.split_once(':') {
            Some((frame_token, body)) => (frame_token.trim(), body.trim()),
            None => {
                return Err(vec![ControlParseError {
                    line: line_number,
                    column: None,
                    message: format!(
                        "frame line `{line}` is missing the `:` separator (expected `<frame>:<keys>`)"
                    ),
                }]);
            }
        };

        let frame_index = match frame_token.parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                return Err(vec![ControlParseError {
                    line: line_number,
                    column: Some(frame_token.len()),
                    message: format!(
                        "frame index `{frame_token}` is not a non-negative integer (leading zeros are allowed)"
                    ),
                }]);
            }
        };

        let mut actions: Vec<WorldActionControl> = Vec::new();

        if !body.is_empty() {
            for pair in body.split(',') {
                let trimmed = pair.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let (key, value) = match trimmed.split_once('=') {
                    Some((key, value)) => (key.trim(), value.trim()),
                    None => {
                        return Err(vec![ControlParseError {
                            line: line_number,
                            column: None,
                            message: format!(
                                "token `{trimmed}` is missing `=` (expected `<key>=<value>`)"
                            ),
                        }]);
                    }
                };
                let key = key.to_ascii_lowercase();
                if key.is_empty() {
                    return Err(vec![ControlParseError {
                        line: line_number,
                        column: None,
                        message: "key before `=` is empty".to_string(),
                    }]);
                }
                let value = match value.parse::<f32>() {
                    Ok(value) => value,
                    Err(_) => {
                        return Err(vec![ControlParseError {
                            line: line_number,
                            column: None,
                            message: format!("value `{value}` for key `{key}` is not a finite f32"),
                        }]);
                    }
                };
                if value.is_nan() {
                    return Err(vec![ControlParseError {
                        line: line_number,
                        column: None,
                        message: format!("value for key `{key}` is NaN"),
                    }]);
                }
                if !(0.0..=1.0).contains(&value) {
                    return Err(vec![ControlParseError {
                        line: line_number,
                        column: None,
                        message: format!(
                            "value {value} for key `{key}` is outside the range [0.0, 1.0]"
                        ),
                    }]);
                }
                actions.push(WorldActionControl::new(key, value, frame_index));
            }
        }

        // The world_model crate stores the per-frame index on `WorldControl` as
        // `frame_count`; we use the parsed frame index so the type round-trips
        // faithfully to the input.
        Ok((
            frame_token.to_string(),
            WorldControl::new(actions, frame_index),
        ))
    }
}

// ---------------------------------------------------------------------------
// Cross-frame validation
// ---------------------------------------------------------------------------

fn validate_sequence(
    frames: &[(usize, String, WorldControl)],
) -> Result<(), Vec<ControlParseError>> {
    if frames.len() < 2 {
        return Ok(());
    }

    let mut errors: Vec<ControlParseError> = Vec::new();

    // Frame-count padding: the raw frame tokens must share the same zero-padded
    // width so that on-disk filenames sort lexicographically. We compare the
    // raw token length rather than the parsed integer width, since the width
    // determines sort order independent of the integer value.
    let first_width = frames[0].1.len();
    for (line_number, raw_token, _) in frames.iter().skip(1) {
        let width = raw_token.len();
        if width != first_width {
            errors.push(ControlParseError {
                line: *line_number,
                column: Some(raw_token.len()),
                message: format!(
                    "frame padding width {width} does not match frame 1 width {first_width}"
                ),
            });
        }
    }

    // Frame ordering must not go backwards.
    for window in frames.windows(2) {
        let (_, _, prev_frame) = &window[0];
        let (next_line, _, next_frame) = &window[1];
        if next_frame.frame_count < prev_frame.frame_count {
            errors.push(ControlParseError {
                line: *next_line,
                column: None,
                message: format!(
                    "frame_count {} precedes earlier frame {} (frames must be monotonically non-decreasing)",
                    next_frame.frame_count, prev_frame.frame_count
                ),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Per-frame semantic validation
// ---------------------------------------------------------------------------

/// Validate WASD/IJKL opposing-key semantics for a single frame's actions.
///
/// Errors are descriptions of the constraint violation. Returns an empty vec
/// if the frame is well-formed.
pub fn validate_frame_semantics(actions: &[WorldActionControl]) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(error) = reject_opposing_pair(actions, MOVE_OPPOSING_PAIRS, "move") {
        errors.push(error);
    }
    if let Some(error) = reject_opposing_pair(actions, LOOK_OPPOSING_PAIRS, "look") {
        errors.push(error);
    }

    for action in actions {
        if action.value.is_nan() {
            errors.push(format!("Action '{}' has NaN value", action.name));
        } else if !(0.0..=1.0).contains(&action.value) {
            errors.push(format!(
                "Action '{}' value {} is outside [0.0, 1.0]",
                action.name, action.value
            ));
        }
    }

    errors
}

fn reject_opposing_pair(
    actions: &[WorldActionControl],
    pairs: &[(&str, &str)],
    group_label: &str,
) -> Option<String> {
    for (low, high) in pairs {
        let low_active = actions
            .iter()
            .any(|action| action.name == *low && action.value != 0.0);
        let high_active = actions
            .iter()
            .any(|action| action.name == *high && action.value != 0.0);
        if low_active && high_active {
            return Some(format!(
                "{group_label} keys `{low}` and `{high}` are both pressed with non-zero values on the same frame"
            ));
        }
    }
    None
}
