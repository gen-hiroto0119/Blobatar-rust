use blobatar_export::Motion;

use crate::editor::EditorState;

pub fn snippet(state: &EditorState) -> String {
    // Debug string literals use Rust's escapes, including \u{...}, not JSON's \u....
    let mut options = serde_json::to_value(&state.settings.options).expect("validated options");
    let defaults =
        serde_json::to_value(blobatar_core::Options::default()).expect("default options");
    options
        .as_object_mut()
        .expect("options object")
        .retain(|key, value| defaults.get(key) != Some(value));
    let options = serde_json::to_string_pretty(&options).expect("options JSON");
    let mut hashes = String::from("#");
    while options.contains(&format!("\"{hashes}")) {
        hashes.push('#');
    }
    let options_literal = format!("r{hashes}\"{options}\"{hashes}");
    let motion = match state.settings.motion {
        Motion::Off => "Never",
        Motion::Hover => "Hover",
        Motion::Always => "Always",
    };
    format!(
        "use blobatar_core::Options;\nuse blobatar_gpui::{{Animate, AnimatedBlobatar}};\n\nlet options: Options = serde_json::from_str({options_literal})?;\nlet avatar = cx.new(|cx| {{\n    let mut avatar = AnimatedBlobatar::new({:?}, &options).size(192.0);\n    avatar.set_animate(Animate::{motion}, cx);\n    avatar.set_reduced_motion({}, cx);\n    avatar\n}});",
        state.seed(),
        state.settings.reduced_motion
    )
}
