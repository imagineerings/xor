use crate::{
    WorldActionControl, WorldControl,
    controls::{
        ControlKeyGroup, ControlParseError, WorldActionControlParser, validate_frame_semantics,
    },
};

fn parse_ok(input: &str) -> Vec<WorldControl> {
    WorldActionControlParser::parse(input).expect("expected successful parse")
}

fn parse_err(input: &str) -> Vec<ControlParseError> {
    WorldActionControlParser::parse(input).expect_err("expected parse errors")
}

// ---------------------------------------------------------------------------
// Empty / comment handling
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_returns_no_frames() {
    let frames = parse_ok("");
    assert!(frames.is_empty());
}

#[test]
fn parse_skips_comments_and_blank_lines() {
    let input = "# header\n\n   \n# trailing\n";
    let frames = parse_ok(input);
    assert!(frames.is_empty());
}

// ---------------------------------------------------------------------------
// Single-frame parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_single_frame_wasd() {
    let frames = parse_ok("0:w=1.0,d=0.5\n");
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert_eq!(frame.frame_count, 0);
    assert_eq!(frame.actions.len(), 2);
    let w = frame
        .actions
        .iter()
        .find(|action| action.name == "w")
        .expect("missing w action");
    assert_eq!(w.value, 1.0);
    let d = frame
        .actions
        .iter()
        .find(|action| action.name == "d")
        .expect("missing d action");
    assert_eq!(d.value, 0.5);
}

#[test]
fn parse_case_insensitive_keys() {
    let frames = parse_ok("0:W=1.0,D=0.5\n");
    let frame = &frames[0];
    assert!(frame.actions.iter().any(|action| action.name == "w"));
    assert!(frame.actions.iter().any(|action| action.name == "d"));
}

#[test]
fn parse_ijkl_camera_keys() {
    let frames = parse_ok("0:i=1.0,l=0.25\n");
    let frame = &frames[0];
    assert!(frame.actions.iter().any(|action| action.name == "i"));
    assert!(frame.actions.iter().any(|action| action.name == "l"));
}

#[test]
fn parse_body_only_with_colon_separator_empty_body() {
    let frames = parse_ok("5:\n");
    assert_eq!(frames.len(), 1);
    assert!(frames[0].actions.is_empty());
    assert_eq!(frames[0].frame_count, 5);
}

#[test]
fn parse_skips_comma_only_body() {
    let frames = parse_ok("0: , , \n");
    assert_eq!(frames.len(), 1);
    assert!(frames[0].actions.is_empty());
}

// ---------------------------------------------------------------------------
// Multi-frame parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_multiple_frames_preserve_indices() {
    let frames = parse_ok("0:w=1.0\n1:w=1.0,d=0.5\n2:w=0.0\n");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].frame_count, 0);
    assert_eq!(frames[1].frame_count, 1);
    assert_eq!(frames[2].frame_count, 2);
}

#[test]
fn parse_accepts_zero_padded_indices() {
    let frames = parse_ok("000:w=1.0\n001:w=1.0\n010:w=0.0\n");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].frame_count, 0);
    assert_eq!(frames[1].frame_count, 1);
    assert_eq!(frames[2].frame_count, 10);
}

#[test]
fn parse_accepts_consistent_no_padding() {
    let frames = parse_ok("0:w=1.0\n1:w=1.0\n2:w=0.0\n");
    assert_eq!(frames.len(), 3);
}

// ---------------------------------------------------------------------------
// Error reporting
// ---------------------------------------------------------------------------

#[test]
fn parse_missing_separator_reports_error() {
    let errors = parse_err("0w=1.0\n");
    assert!(errors.iter().any(|error| error.message.contains(":")));
}

#[test]
fn parse_invalid_frame_index_reports_error() {
    let errors = parse_err("abc:w=1.0\n");
    assert!(errors.iter().any(|error| error.message.contains("integer")));
}

#[test]
fn parse_missing_value_assignments_reports_error() {
    let errors = parse_err("0:w\n");
    assert!(errors.iter().any(|error| error.message.contains("=")));
}

#[test]
fn parse_rejects_nan_values_explicitly() {
    let errors = parse_err("0:w=NaN\n");
    assert!(errors.iter().any(|error| error.message.contains("NaN")));
}

#[test]
fn parse_rejects_out_of_range_values() {
    let errors = parse_err("0:w=1.5\n");
    assert!(errors.iter().any(|error| error.message.contains("range")));
}

#[test]
fn parse_rejects_negative_values() {
    let errors = parse_err("0:w=-0.25\n");
    assert!(errors.iter().any(|error| error.message.contains("range")));
}

#[test]
fn parse_rejects_empty_key_before_equals() {
    let errors = parse_err("0:=1.0\n");
    assert!(errors.iter().any(|error| error.message.contains("empty")));
}

#[test]
fn parse_rejects_non_numeric_value() {
    let errors = parse_err("0:w=high\n");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("finite f32"))
    );
}

#[test]
fn parse_rejects_unknown_token_missing_equals() {
    let errors = parse_err("0:jump\n");
    assert!(errors.iter().any(|error| error.message.contains("=")));
}

// ---------------------------------------------------------------------------
// Cross-frame validation
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_backwards_frame_order() {
    let errors = parse_err("1:w=1.0\n0:w=1.0\n");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("monotonically"))
    );
}

#[test]
fn parse_rejects_inconsistent_padding_width() {
    let errors = parse_err("001:w=1.0\n02:w=1.0\n");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("padding width"))
    );
}

// ---------------------------------------------------------------------------
// Semantic validation (per-frame)
// ---------------------------------------------------------------------------

#[test]
fn control_key_group_classification() {
    for key in ["w", "a", "s", "d"] {
        assert_eq!(ControlKeyGroup::classify(key), ControlKeyGroup::Move);
    }
    for key in ["i", "j", "k", "l"] {
        assert_eq!(ControlKeyGroup::classify(key), ControlKeyGroup::Look);
    }
    assert_eq!(ControlKeyGroup::classify("x"), ControlKeyGroup::Unknown);

    assert_eq!(ControlKeyGroup::Move.label(), "move");
    assert_eq!(ControlKeyGroup::Look.label(), "look");
    assert_eq!(ControlKeyGroup::Unknown.label(), "unknown");
}

#[test]
fn frame_semantics_reject_w_and_s_active_together() {
    let actions = vec![
        WorldActionControl::new("w", 1.0, 0),
        WorldActionControl::new("s", 0.5, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(
        errors
            .iter()
            .any(|error| error.contains('w') && error.contains('s'))
    );
}

#[test]
fn frame_semantics_reject_a_and_d_active_together() {
    let actions = vec![
        WorldActionControl::new("a", 1.0, 0),
        WorldActionControl::new("d", 0.5, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(
        errors
            .iter()
            .any(|error| error.contains('a') && error.contains('d'))
    );
}

#[test]
fn frame_semantics_reject_i_and_k_active_together() {
    let actions = vec![
        WorldActionControl::new("i", 1.0, 0),
        WorldActionControl::new("k", 1.0, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(
        errors
            .iter()
            .any(|error| error.contains('i') && error.contains('k'))
    );
}

#[test]
fn frame_semantics_reject_j_and_l_active_together() {
    let actions = vec![
        WorldActionControl::new("j", 0.75, 0),
        WorldActionControl::new("l", 0.25, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(
        errors
            .iter()
            .any(|error| error.contains('j') && error.contains('l'))
    );
}

#[test]
fn frame_semantics_allow_zero_value_with_opposite() {
    // If one of the opposing keys is set to 0.0, the pair is fine.
    let actions = vec![
        WorldActionControl::new("w", 1.0, 0),
        WorldActionControl::new("s", 0.0, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[test]
fn frame_semantics_reject_nan_and_out_of_range() {
    let actions = vec![
        WorldActionControl::new("w", f32::NAN, 0),
        WorldActionControl::new("d", 1.5, 0),
    ];
    let errors = validate_frame_semantics(&actions);
    assert!(errors.iter().any(|error| error.contains("NaN")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("1.5") || error.contains("outside"))
    );
}

// ---------------------------------------------------------------------------
// Value-range acceptance
// ---------------------------------------------------------------------------

#[test]
fn parse_accepts_zero_value_and_simultaneous_opposite() {
    let frames = parse_ok("0:w=0.0,s=0.0\n");
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    let errors = validate_frame_semantics(&frame.actions);
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[test]
fn parse_accepts_unknown_group_keys() {
    // `space` is an Unknown group key — it does not participate in opposing
    // pair validation but is still structurally counted.
    let frames = parse_ok("0:w=1.0,space=1.0\n");
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert!(frame.actions.iter().any(|action| action.name == "space"));
    let errors = validate_frame_semantics(&frame.actions);
    assert!(errors.is_empty(), "got: {errors:?}");
}

// ---------------------------------------------------------------------------
// Integration with `WorldControl::validate`
// ---------------------------------------------------------------------------

#[test]
fn round_trip_world_control_validate_passes_for_well_formed_input() {
    let frames = parse_ok("0:w=1.0,d=0.5\n1:w=1.0,d=0.5,j=0.25\n");
    for frame in &frames {
        let errors = frame.validate();
        assert!(errors.is_empty(), "got: {errors:?}");
    }
}
