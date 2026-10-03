use blobatar_core::{
    pose::Expression,
    traits::{TraitOverrides, Traits},
};
use blobatar_motion::{
    gaze,
    idle::{IdleSeeds, idle_at},
};
use serde_json::{Value, json};

fn close(actual: &Value, expected: &Value) {
    match expected {
        Value::Number(number) => {
            let expected = number.as_f64().unwrap();
            let actual = actual.as_f64().unwrap();
            let tolerance = 1e-12_f64.max(1e-9 * actual.abs().max(expected.abs()));
            assert!(
                (actual - expected).abs() <= tolerance,
                "{actual} != {expected}"
            );
        }
        Value::Array(items) => {
            assert_eq!(actual.as_array().unwrap().len(), items.len());
            for (index, item) in items.iter().enumerate() {
                close(&actual[index], item);
            }
        }
        Value::Object(items) => {
            for (key, item) in items {
                close(&actual[key], item);
            }
        }
        _ => assert_eq!(actual, expected),
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/motion-2.7.0.json")).unwrap()
}

fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap()
}

#[test]
fn expressions_and_morphs_match_upstream() {
    let fixture = fixture();
    for case in fixture["poses"].as_array().unwrap() {
        let expression: Expression = serde_json::from_value(case["name"].clone()).unwrap();
        close(&json!(expression.pose()), &case["pose"]);
    }
    for case in fixture["morphs"].as_array().unwrap() {
        let from: Expression = serde_json::from_value(case["from"].clone()).unwrap();
        let to: Expression = serde_json::from_value(case["to"].clone()).unwrap();
        close(
            &json!(from.pose().interpolate(to.pose(), number(case, "t"))),
            &case["pose"],
        );
    }
}

#[test]
fn idle_frames_match_upstream() {
    let fixture = fixture();
    let overrides = TraitOverrides::new();
    for case in fixture["idle"].as_array().unwrap() {
        let traits = Traits::new(case["name"].as_str().unwrap(), true, &overrides);
        let seeds = IdleSeeds::new(&traits);
        close(&json!(seeds), &case["seeds"]);
        close(
            &json!(idle_at(
                seeds,
                number(case, "time"),
                number(case, "amplitude"),
                number(case, "shake")
            )),
            &case["frame"],
        );
    }
}

#[test]
fn gaze_math_matches_upstream() {
    let fixture = fixture();
    for case in fixture["projections"].as_array().unwrap() {
        close(
            &json!(gaze::project(
                serde_json::from_value(case["mark"].clone()).unwrap(),
                number(case, "yaw"),
                number(case, "pitch")
            )),
            &case["result"],
        );
    }
    for case in fixture["steps"].as_array().unwrap() {
        close(
            &json!(gaze::step(
                serde_json::from_value(case["input"].clone()).unwrap()
            )),
            &case["result"],
        );
    }
    for case in fixture["thresholds"].as_array().unwrap() {
        close(
            &json!(gaze::threshold(
                number(case, "width"),
                number(case, "travel")
            )),
            &case["value"],
        );
    }
    for case in fixture["pursuits"].as_array().unwrap() {
        close(
            &json!(gaze::pursuit(number(case, "dt"), number(case, "settle"))),
            &case["value"],
        );
    }
}

#[test]
fn transformed_neutral_paths_match_upstream_composition() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/transforms-2.7.0.json")).unwrap();
    assert_eq!(fixture["meta"]["caseCount"], 6048);
    for case in fixture["cases"].as_array().unwrap() {
        let seed = case["seed"].as_str().unwrap();
        let options: blobatar_core::Options =
            serde_json::from_value(case["options"].clone()).unwrap();
        let expression: Expression = serde_json::from_value(case["expression"].clone()).unwrap();
        let avatar = blobatar_core::Avatar::new(seed, &options);
        let traits = Traits::new(seed, options.normalize, &options.traits);
        let pose = expression.pose();
        let idle = idle_at(
            IdleSeeds::new(&traits),
            number(case, "time"),
            number(case, "amplitude"),
            pose.shake,
        );
        close(
            &json!(blobatar_motion::transform::frame_transforms(
                &avatar.layout,
                pose,
                idle
            )),
            &case["transforms"],
        );
    }
}
