use serde_json::json;

use crate::{
    SIM_UTILITY_DATASET_PATH_CODE, SIM_UTILITY_INVALID_REGEX_CODE,
    SIM_UTILITY_MATH_EXPRESSION_CODE, SimDatasetEntry, SimUtilityLogicOp, SimUtilityNodeAdapter,
    SimUtilityValue,
};

#[test]
fn utility_adapter_handles_primitives_regex_json_math_logic_and_switches() {
    let adapter = SimUtilityNodeAdapter::new();
    assert_eq!(
        adapter.string("prompt"),
        SimUtilityValue::String("prompt".to_string())
    );
    assert_eq!(adapter.number(4.0), SimUtilityValue::Number(4.0));
    assert_eq!(adapter.boolean(true), SimUtilityValue::Boolean(true));
    assert_eq!(adapter.seed(42), SimUtilityValue::Seed(42));

    let captures = adapter
        .regex_extract(r"item-(\d+)", "item-10 item-20")
        .expect("regex extract");
    assert_eq!(captures, vec!["10".to_string(), "20".to_string()]);

    let value = json!({"items": [{"name": "first"}, {"name": "second"}]});
    assert_eq!(
        adapter
            .json_extract(&value, "items.1.name")
            .expect("json path"),
        json!("second")
    );

    assert_eq!(adapter.math_binary(6.0, "*", 7.0).expect("math"), 42.0);
    assert!(adapter.logic(SimUtilityLogicOp::And, &[true, true]));
    assert!(adapter.logic(SimUtilityLogicOp::Xor, &[true, false, false]));
    assert_eq!(adapter.switch(true, "left", "right"), "left");
}

#[test]
fn utility_adapter_reports_invalid_regex_and_math() {
    let adapter = SimUtilityNodeAdapter::new();
    let regex = adapter
        .regex_extract("(", "input")
        .expect_err("invalid regex");
    assert_eq!(regex.code, SIM_UTILITY_INVALID_REGEX_CODE);

    let math = adapter
        .math_binary(1.0, "/", 0.0)
        .expect_err("division by zero");
    assert_eq!(math.code, SIM_UTILITY_MATH_EXPRESSION_CODE);
}

#[test]
fn utility_adapter_prepares_path_confined_dataset_entries() {
    let adapter = SimUtilityNodeAdapter::new();
    let first = SimDatasetEntry::new("inputs/a.png", "asset://a")
        .expect("entry")
        .with_text("red chair")
        .with_bucket("chairs")
        .with_attribution("source", "artist");
    let second = SimDatasetEntry::new("inputs/b.png", "asset://b")
        .expect("entry")
        .with_text("blue table")
        .with_bucket("tables");
    let duplicate = SimDatasetEntry::new("inputs/a.png", "asset://a")
        .expect("entry")
        .with_text("red chair")
        .with_bucket("chairs");

    let prepared = adapter
        .prepare_dataset(&[first, second, duplicate])
        .expect("prepared");
    assert_eq!(prepared.len(), 3);

    let deduplicated = adapter.dataset_deduplicate(&prepared);
    assert_eq!(deduplicated.len(), 2);

    let shuffled_once = adapter.dataset_shuffle(&deduplicated, 7);
    let shuffled_twice = adapter.dataset_shuffle(&deduplicated, 7);
    assert_eq!(shuffled_once, shuffled_twice);

    let buckets = adapter.dataset_buckets(&prepared);
    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0].key, "chairs");
    assert_eq!(buckets[0].entries.len(), 2);
}

#[test]
fn utility_adapter_rejects_dataset_path_escape() {
    let diagnostic = SimDatasetEntry::new("../outside.png", "asset://outside")
        .expect_err("path escape rejected");
    assert_eq!(diagnostic.code, SIM_UTILITY_DATASET_PATH_CODE);
}
