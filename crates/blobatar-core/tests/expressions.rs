use blobatar_core::{Avatar, Expression, Options};
use serde_json::{Value, json};

#[test]
fn all_shapes_expressions_seeds_and_tones_match_reference() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/expressions-2.7.0.json")).unwrap();
    assert_eq!(fixture["meta"]["caseCount"], 3360);
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let options: Options = serde_json::from_value(case["options"].clone()).unwrap();
        let avatar = Avatar::new(case["seed"].as_str().unwrap(), &options);
        assert_eq!(json!(avatar.palette), case["palette"], "palette {index}");
        let actual_eyes = json!(avatar.layout.eyes);
        assert_eq!(
            actual_eyes.as_array().unwrap().len(),
            case["eyes"].as_array().unwrap().len()
        );
        for (actual, expected) in actual_eyes
            .as_array()
            .unwrap()
            .iter()
            .zip(case["eyes"].as_array().unwrap())
        {
            for (channel, value) in expected.as_object().unwrap() {
                let a = actual[channel].as_f64().unwrap();
                let b = value.as_f64().unwrap();
                assert!(
                    (a - b).abs() <= 1e-12_f64.max(1e-9 * a.abs().max(b.abs())),
                    "eye {index}/{channel}"
                );
            }
        }
        assert_eq!(
            avatar.svg(&options),
            case["svg"].as_str().unwrap(),
            "SVG {index}"
        );
    }
    for case in fixture["overrides"].as_array().unwrap() {
        let options: Options = serde_json::from_value(case["options"].clone()).unwrap();
        let avatar = Avatar::new(case["seed"].as_str().unwrap(), &options);
        assert_eq!(avatar.svg(&options), case["svg"].as_str().unwrap());
        assert_eq!(avatar.uri(&options), case["uri"].as_str().unwrap());
    }
}

#[test]
fn idle_is_byte_identical_to_an_omitted_expression() {
    let options = Options::default();
    let idle = Options {
        expression: Some(Expression::Idle),
        ..options.clone()
    };
    assert_eq!(
        Avatar::new("ひろと", &options).svg(&options),
        Avatar::new("ひろと", &idle).svg(&idle)
    );
}
