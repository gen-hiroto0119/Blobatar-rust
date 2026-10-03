use blobatar_core::traits::Override;
use blobatar_export::Motion;

use crate::editor::EditorState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Api {
    Rust,
    React,
    Vue,
    Svelte,
    Solid,
    Preact,
    ReactNative,
    String,
    Http,
}

impl Api {
    pub const ALL: [Self; 9] = [
        Self::Rust,
        Self::React,
        Self::Vue,
        Self::Svelte,
        Self::Solid,
        Self::Preact,
        Self::ReactNative,
        Self::String,
        Self::Http,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "Rust/GPUI",
            Self::React => "react",
            Self::Vue => "vue",
            Self::Svelte => "svelte",
            Self::Solid => "solid",
            Self::Preact => "preact",
            Self::ReactNative => "react-native",
            Self::String => "string",
            Self::Http => "http",
        }
    }
}

const NAME_NOTE: &str = "everything below comes from the name unless it is pinned";

pub fn snippet(api: Api, state: &EditorState, endpoint: &str) -> String {
    match api {
        Api::Rust => rust(state),
        Api::String => string(state),
        Api::Http => http(state, endpoint),
        _ => component(api, state),
    }
}

fn json(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize")
}

fn object_key(api: Api, key: &str) -> String {
    let mut chars = key.chars();
    let bare = chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_' || ch == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$');
    if bare {
        key.into()
    } else if api == Api::Vue {
        format!("'{}'", key.replace('\\', "\\\\").replace('\'', "\\'"))
    } else {
        json(key)
    }
}

fn literal(value: &Override) -> String {
    match value {
        Override::Fixed(value) => number(*value),
        Override::Candidates(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| number(*value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

fn attr_string(api: Api, name: &str, value: &str) -> String {
    if api == Api::Vue {
        format!(
            "{name}=\"{}\"",
            value.replace('&', "&amp;").replace('"', "&quot;")
        )
    } else if value.contains(['"', '\\']) {
        format!("{name}={{{}}}", json(value))
    } else {
        format!("{name}=\"{value}\"")
    }
}

fn attr_expr(api: Api, name: &str, expr: &str) -> String {
    if api == Api::Vue {
        format!(":{name}=\"{expr}\"")
    } else {
        format!("{name}={{{expr}}}")
    }
}

fn component(api: Api, state: &EditorState) -> String {
    let native = api == Api::ReactNative;
    let moving = state.settings.motion != Motion::Off;
    let tag = if native && moving {
        "AnimatedBlobatar"
    } else {
        "Blobatar"
    };
    let package = format!(
        "@blobatar/{}{}",
        api.name(),
        if native && moving { "/animated" } else { "" }
    );
    let mut imports = vec![format!("import {{ {tag} }} from \"{package}\";")];
    if moving && !native {
        imports.push(
            "import \"blobatar/motion.css\"; // animate renders inline SVG, not one <img>".into(),
        );
    }
    let pins = state.ordered_pins();
    let template = matches!(api, Api::Vue | Api::Svelte);
    let mut lines = Vec::new();
    if !pins.is_empty() {
        lines.push(if template {
            format!("<!-- {NAME_NOTE} -->")
        } else {
            format!("// {NAME_NOTE}")
        });
    }
    lines.push(format!("<{tag}"));
    lines.push(format!("  {}", attr_string(api, "name", state.seed())));
    if native {
        lines.push(format!("  {}", attr_expr(api, "size", "48")));
    }
    if let [(key, value)] = pins.as_slice() {
        let object = format!("{{ {}: {} }}", object_key(api, key), literal(value));
        lines.push(format!("  {}", attr_expr(api, "traits", &object)));
    } else if !pins.is_empty() {
        lines.push(
            if api == Api::Vue {
                "  :traits=\"{"
            } else {
                "  traits={{"
            }
            .into(),
        );
        for (key, value) in pins {
            lines.push(format!("    {}: {},", object_key(api, key), literal(value)));
        }
        lines.push(if api == Api::Vue { "  }\"" } else { "  }}" }.into());
    }
    if moving {
        lines.push(if native {
            "  animate // no :hover on a touch screen, so this one is yours to drive".into()
        } else {
            format!(
                "  {}",
                attr_string(
                    api,
                    "animate",
                    if state.settings.motion == Motion::Always {
                        "always"
                    } else {
                        "hover"
                    }
                )
            )
        });
    }
    lines.push(if template { "/>" } else { "/>;" }.into());
    match api {
        Api::Vue => format!(
            "<script setup>\n{}\n</script>\n\n<template>\n{}\n</template>",
            imports.join("\n"),
            lines
                .iter()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        Api::Svelte => format!(
            "<script>\n{}\n</script>\n\n{}",
            imports
                .iter()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
            lines.join("\n")
        ),
        _ => format!("{}\n\n{}", imports.join("\n"), lines.join("\n")),
    }
}

fn string(state: &EditorState) -> String {
    let mut lines = vec![
        "import { blobatar } from \"blobatar\";".into(),
        String::new(),
    ];
    if state.settings.motion != Motion::Off {
        lines.push("// animate is a component option — this renders static markup".into());
    }
    let pins = state.ordered_pins();
    if !pins.is_empty() {
        lines.push(format!("// {NAME_NOTE}"));
    }
    let call = format!("const svg = blobatar({}", json(state.seed()));
    if pins.is_empty() {
        lines.push(format!("{call});"));
    } else {
        lines.push(format!("{call}, {{"));
        lines.push("  traits: {".into());
        for (key, value) in pins {
            lines.push(format!(
                "    {}: {},",
                object_key(Api::String, key),
                literal(value)
            ));
        }
        lines.push("  },".into());
        lines.push("});".into());
    }
    lines.join("\n")
}

fn uri_component(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            result.push(char::from(byte));
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

fn http(state: &EditorState, endpoint: &str) -> String {
    let mut query = vec!["gen=2".into()];
    let mut unspellable = Vec::new();
    let mut narrowed = Vec::new();
    for (key, value) in state.ordered_pins() {
        match value {
            Override::Candidates(_) => narrowed.push(key),
            Override::Fixed(value) if key == "hue" => query.push(format!(
                "hue={}",
                number((value * 36000.0 + 0.5).floor() / 100.0)
            )),
            Override::Fixed(value) if key == "tone" => {
                query.push(format!("tone={}", number(*value)))
            }
            _ => unspellable.push(key),
        }
    }
    let mut lines = Vec::new();
    if !unspellable.is_empty() {
        lines.push(format!(
            "# no url spelling for {} — from the name",
            unspellable.join(", ")
        ));
    }
    if !narrowed.is_empty() {
        lines.push(format!(
            "# {} narrowed — a url states one value, so this is from the name too",
            narrowed.join(", ")
        ));
    }
    if state.settings.motion != Motion::Off {
        lines.push("# static svg — animate is a component option".into());
    }
    lines.push(format!(
        "{}/{}?{}",
        endpoint.trim_end_matches('/'),
        uri_component(state.seed()),
        query.join("&")
    ));
    lines.join("\n")
}

fn rust(state: &EditorState) -> String {
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
