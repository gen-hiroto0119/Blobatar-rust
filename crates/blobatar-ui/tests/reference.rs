use blobatar_core::traits::Override;
use blobatar_export::Motion;
use blobatar_ui::{
    axes::{self, AXES, SHAPES, TONES},
    editor::EditorState,
    snippet,
};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/editor-2.7.0.json")).unwrap()
}

fn near(actual: f64, expected: f64) {
    let tolerance = 1e-12_f64.max(1e-9 * actual.abs().max(expected.abs()));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn axes_and_all_picker_subsets_match_upstream() {
    let fixture = fixture();
    assert_eq!(serde_json::to_value(AXES).unwrap(), fixture["axes"]);
    assert_eq!(serde_json::to_value(SHAPES).unwrap(), fixture["shapes"]);
    assert_eq!(serde_json::to_value(TONES).unwrap(), fixture["tones"]);
    for case in fixture["toggles"].as_array().unwrap() {
        let choices = if case["key"] == "shape" {
            SHAPES.as_slice()
        } else {
            TONES.as_slice()
        };
        let chosen: Vec<f64> = serde_json::from_value(case["chosen"].clone()).unwrap();
        let next = axes::toggle_choice(choices, &chosen, case["at"].as_f64().unwrap());
        assert_eq!(serde_json::to_value(&next).unwrap(), case["next"]);
        assert_eq!(
            serde_json::to_value(axes::narrow_pin(next)).unwrap(),
            case["pin"]
        );
    }
    for case in fixture["bands"].as_array().unwrap() {
        assert_eq!(
            axes::band_index(
                case["value"].as_f64().unwrap(),
                case["count"].as_u64().unwrap() as usize
            ),
            case["index"].as_u64().unwrap() as usize
        );
    }
}

#[test]
fn editor_readback_and_locks_match_upstream() {
    for (index, case) in fixture()["states"].as_array().unwrap().iter().enumerate() {
        let mut state = EditorState::default();
        state.settings.name = case["name"].as_str().unwrap().into();
        state
            .apply_traits_json(&case["pinned"].to_string())
            .unwrap();
        let layout = state.neutral_layout();
        assert_eq!(
            layout.shape,
            case["shape"].as_str().unwrap(),
            "case {index}"
        );
        assert_eq!(
            serde_json::to_value(axes::candidates(
                state.settings.options.traits.get("shape"),
                &layout.shape
            ))
            .unwrap(),
            case["shapes"]
        );
        assert_eq!(
            serde_json::to_value(
                state
                    .applicable_axes()
                    .iter()
                    .map(|axis| axis.key)
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            case["applicable"]
        );
        if case["name"].as_str() == Some("") {
            continue;
        }
        let ghosts = state.fit_readback();
        assert_eq!(
            ghosts.len(),
            case["ghosts"].as_object().unwrap().len(),
            "case {index}"
        );
        for (key, value) in ghosts {
            near(value, case["ghosts"][key].as_f64().unwrap());
        }
        for axis in AXES {
            let mut pinned = state.clone();
            pinned.pin(axis.key, state.reader().get(axis.key));
            match pinned.settings.options.traits[axis.key] {
                Override::Fixed(value) => near(value, case["pins"][axis.key].as_f64().unwrap()),
                _ => panic!("pin must be scalar"),
            }
        }
    }
}

#[test]
fn empty_name_uses_blobatar_reader_in_both_generations() {
    for generation in [1, 2] {
        let mut empty = EditorState::default();
        empty.settings.generation = generation;
        empty.settings.name.clear();
        let mut named = empty.clone();
        named.settings.name = "blobatar".into();

        for axis in AXES {
            assert_eq!(
                empty.reader().get(axis.key),
                named.reader().get(axis.key),
                "generation {generation}, {}",
                axis.key
            );
            empty.toggle_lock(axis.key);
            named.toggle_lock(axis.key);
        }
        assert_eq!(
            serde_json::to_value(&empty.settings.options.traits).unwrap(),
            serde_json::to_value(&named.settings.options.traits).unwrap()
        );
        assert_eq!(empty.fit_readback(), named.fit_readback());
        assert_eq!(
            empty.avatar().svg(&empty.settings.options),
            named.avatar().svg(&named.settings.options)
        );
        assert_eq!(empty.settings.svg().unwrap(), named.settings.svg().unwrap());
        assert_eq!(snippet::snippet(&empty), snippet::snippet(&named));
    }
}

#[test]
fn selection_shuffle_unlock_and_reset_share_one_override_map() {
    let mut state = EditorState::default();
    for choice in SHAPES.iter().rev() {
        state.toggle_choice("shape", &SHAPES, choice.at);
    }
    assert!(
        matches!(&state.settings.options.traits["shape"], Override::Candidates(values) if values.len() == 10)
    );
    state.toggle_lock("eye.gap");
    let before = serde_json::to_value(&state.settings.options.traits).unwrap();
    state.shuffle_to("別の名前".into());
    assert_eq!(
        serde_json::to_value(&state.settings.options.traits).unwrap(),
        before
    );
    state.toggle_lock("eye.gap");
    assert!(!state.settings.options.traits.contains_key("eye.gap"));
    state.reset();
    assert!(state.settings.options.traits.is_empty());
    assert_eq!(state.settings.name, "別の名前");
    assert!(state.apply_traits_json("{\"shape\": [\"bad\"]}").is_err());
    assert!(state.settings.options.traits.is_empty());
}

#[test]
fn rust_snippet_serialized_options_reproduce_the_preview() {
    use blobatar_core::{Avatar, Background, Expression, Options};
    for expression in Expression::ALL {
        let mut state = EditorState::default();
        state.settings.name = "ひろと \"# 🦀\n".into();
        state.settings.options.expression = Some(expression);
        state.settings.options.background = Some(Background::Kind("squircle".into()));
        state.settings.options.title = Some("quoted \"# title".into());
        state
            .apply_traits_json(r#"{"shape":[0.11,0.965],"eye.gap":0.751,"hue":0.123}"#)
            .unwrap();
        let code = snippet::snippet(&state);
        let raw = code.split("serde_json::from_str(r").nth(1).unwrap();
        let (hashes, body) = raw.split_once('"').unwrap();
        let (json, _) = body.split_once(&format!("\"{hashes}")).unwrap();
        let options: Options = serde_json::from_str(json).unwrap();
        assert_eq!(
            Avatar::new(state.seed(), &options).svg(&options),
            state.avatar().svg(&state.settings.options)
        );
    }
    let code = snippet::snippet(&EditorState::default());
    assert!(!code.contains("traits"));
}

#[test]
fn rust_snippet_preserves_name_and_playback_settings() {
    for (motion, mode) in [
        (Motion::Off, "Never"),
        (Motion::Hover, "Hover"),
        (Motion::Always, "Always"),
    ] {
        for reduced in [false, true] {
            for name in ["", "ひろと \"# 🦀\n\\"] {
                let mut state = EditorState::default();
                state.settings.name = name.into();
                state.settings.motion = motion;
                state.settings.reduced_motion = reduced;
                let code = snippet::snippet(&state);
                assert!(code.contains(&format!(
                    "AnimatedBlobatar::with_generation({:?}, &options, Generation::Two)",
                    state.seed()
                )));
                assert!(code.contains(&format!("set_animate(Animate::{mode}, cx)")));
                assert!(code.contains(&format!("set_reduced_motion({reduced}, cx)")));
            }
        }
    }
}

#[test]
fn generation_one_picker_preview_and_snippet_use_the_legacy_shapes() {
    let mut state = EditorState::default();
    state.settings.generation = 1;
    assert_eq!(state.shape_choices().len(), 6);
    for choice in state.shape_choices() {
        state.pin("shape", choice.at);
        assert_eq!(state.avatar().layout.shape, choice.name);
        assert_eq!(state.neutral_layout().shape, choice.name);
    }
    state.pin("shape", 0.65);
    assert_eq!(state.avatar().layout.shape, "boxy");
    assert!(
        !state
            .applicable_axes()
            .iter()
            .any(|axis| axis.key == "capsule.squat")
    );
    let code = snippet::snippet(&state);
    assert!(code.contains("Generation::One"));
    assert_eq!(
        state.settings.svg().unwrap(),
        state.avatar().svg(&state.settings.options)
    );
    state.settings.generation = 2;
    assert_eq!(state.avatar().layout.shape, "capsule");
    assert!(
        state
            .applicable_axes()
            .iter()
            .any(|axis| axis.key == "capsule.squat")
    );
}
