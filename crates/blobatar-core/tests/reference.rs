use blobatar_core::{
    Avatar, Options, color, hash,
    traits::{Override, TraitOverrides, Traits},
};
use serde_json::{Value, json};

fn close(actual: &Value, expected: &Value, path: &str) {
    match expected {
        Value::Number(n) => {
            let e = n.as_f64().unwrap();
            let a = actual
                .as_f64()
                .unwrap_or_else(|| panic!("{path}: expected number, got {actual}"));
            let tolerance = 1e-12_f64.max(1e-9 * a.abs().max(e.abs()));
            assert!((a - e).abs() <= tolerance, "{path}: {a} != {e}");
        }
        Value::Object(o) => {
            for (key, value) in o {
                close(&actual[key], value, &format!("{path}.{key}"));
            }
        }
        Value::Array(values) => {
            assert_eq!(actual.as_array().unwrap().len(), values.len(), "{path}");
            for (i, value) in values.iter().enumerate() {
                close(&actual[i], value, &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

fn corpus(source: &str) {
    let fixture: Value = serde_json::from_str(source).unwrap();
    for case in fixture["hash"].as_array().unwrap() {
        let seed = case["seed"].as_str().unwrap();
        let state = hash::seed_state(seed, case["normalize"].as_bool().unwrap_or(true));
        assert_eq!(
            hash::normalize_seed(seed),
            case["normalized"].as_str().unwrap(),
            "{seed:?}"
        );
        assert_eq!(
            u64::from(state),
            case["state"].as_i64().unwrap() as u32 as u64,
            "{seed:?}"
        );
        for (key, expected) in case["streams"].as_object().unwrap() {
            assert_eq!(
                hash::stream(state, key),
                expected.as_f64().unwrap(),
                "{seed:?}/{key}"
            );
        }
    }
    for (i, case) in fixture["palette"].as_array().unwrap().iter().enumerate() {
        let hue = case["hue"].as_f64().unwrap();
        let tone = case["tone"].as_f64().unwrap();
        assert_eq!(
            json!(color::palette(hue, true, tone)),
            case["hex"],
            "palette {i}"
        );
        close(&json!(color::ramp(hue, true, tone)), &case["ramp"], "ramp");
        close(
            &json!(color::ramp(hue, false, tone)),
            &case["rampUnenforced"],
            "rampUnenforced",
        );
    }
    for (i, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let options: Options = serde_json::from_value(case["options"].clone()).unwrap();
        let avatar = Avatar::new(case["seed"].as_str().unwrap(), &options);
        let layout = json!(avatar.layout);
        for key in ["body", "face", "eyes", "petals"] {
            close(&layout[key], &case[key], &format!("case[{i}].{key}"));
        }
        assert_eq!(layout["shape"], case["shape"], "case {i}");
        assert_eq!(json!(avatar.layout.extra_svg()), case["extra"], "extra {i}");
        assert_eq!(
            json!(avatar.layout.body_path().to_svg()),
            case["bodyPath"],
            "body path {i}"
        );
        assert_eq!(
            json!(
                avatar
                    .layout
                    .eye_paths()
                    .iter()
                    .map(|p| p.to_svg())
                    .collect::<Vec<_>>()
            ),
            case["eyePaths"],
            "eye paths {i}"
        );
        assert_eq!(json!(avatar.palette), case["palette"], "palette {i}");
        if let Some(svg) = case.get("svg") {
            assert_eq!(json!(avatar.svg(&options)), *svg, "SVG {i}");
        }
        if let Some(uri) = case.get("uri") {
            assert_eq!(json!(avatar.uri(&options)), *uri, "URI {i}");
        }
    }
}

#[test]
fn legacy_generation_two() {
    corpus(include_str!("fixtures/gen2-2.4.0.json"));
}

#[test]
fn pinned_generation_two() {
    corpus(include_str!("fixtures/gen2-2.7.0.json"));
}

fn number(v: &Value) -> f64 {
    match v.as_str() {
        Some("NaN") => f64::NAN,
        Some("Infinity") => f64::INFINITY,
        Some("-Infinity") => f64::NEG_INFINITY,
        _ => v.as_f64().unwrap(),
    }
}

#[test]
fn trait_override_boundaries() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/gen2-2.7.0.json")).unwrap();
    for case in fixture["traitCases"].as_array().unwrap() {
        let key = case["key"].as_str().unwrap();
        let input = &case["input"];
        let value = match input.as_array() {
            Some(v) => Override::Candidates(v.iter().map(number).collect()),
            None => Override::Fixed(number(input)),
        };
        let overrides = TraitOverrides::from([(key.to_owned(), value)]);
        assert_eq!(
            Traits::new("edge-seed", true, &overrides).get(key),
            case["value"].as_f64().unwrap(),
            "{case}"
        );
    }
}
