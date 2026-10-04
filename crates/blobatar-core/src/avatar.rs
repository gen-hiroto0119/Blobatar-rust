use serde::{Deserialize, Serialize};

use crate::{
    color::{self, Palette},
    hash::is_ecmascript_whitespace,
    layout::{self, Layout},
    pose::{self, Expression},
    shape::{self, rounded_number},
    traits::TraitOverrides,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Background {
    Enabled(bool),
    Kind(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Options {
    pub size: Option<f64>,
    pub background: Option<Background>,
    pub palette: Option<Palette>,
    pub hue: Option<f64>,
    pub tone: Option<f64>,
    pub traits: TraitOverrides,
    pub normalize: bool,
    pub contrast: bool,
    pub title: Option<String>,
    pub expression: Option<Expression>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            size: None,
            background: None,
            palette: None,
            hue: None,
            tone: None,
            traits: TraitOverrides::new(),
            normalize: true,
            contrast: true,
            title: None,
            expression: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Avatar {
    pub layout: Layout,
    pub palette: Palette,
    pub body_offset_y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Generation {
    One,
    #[default]
    Two,
}

impl Avatar {
    pub fn new(seed: &str, options: &Options) -> Self {
        Self::with_generation(seed, options, Generation::Two)
    }

    pub fn with_generation(seed: &str, options: &Options, generation: Generation) -> Self {
        let layout = match generation {
            Generation::One => {
                layout::make_generation1_layout(seed, options.normalize, &options.traits)
            }
            Generation::Two => layout::make_layout(seed, options.normalize, &options.traits),
        };
        let traits = crate::traits::Traits::new(seed, options.normalize, &options.traits);
        let hue = options.hue.unwrap_or_else(|| traits.num("hue", 0.0, 360.0));
        let tone = options.tone.unwrap_or_else(|| traits.get("tone"));
        let mut palette = color::palette(hue, options.contrast, tone);

        if let Some(override_palette) = &options.palette {
            if override_palette.bg.is_some() {
                palette.bg.clone_from(&override_palette.bg);
            }
            if override_palette.head.is_some() {
                palette.head.clone_from(&override_palette.head);
            }
            if override_palette.eye.is_some() {
                palette.eye.clone_from(&override_palette.eye);
            }
        }

        let expression = options.expression.unwrap_or_default();
        let pose = expression.pose();
        Self {
            layout: pose::bake(&layout, pose),
            palette: expression.palette(&palette),
            body_offset_y: pose.body_offset_y,
        }
    }

    pub fn svg(&self, options: &Options) -> String {
        let mut svg =
            String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"");
        if let Some(size) = options.size.filter(|size| *size != 0.0) {
            let size = size.to_string();
            svg.push_str(&format!(" width=\"{size}\" height=\"{size}\""));
        }
        svg.push('>');

        if let Some(title) = options.title.as_deref().filter(|title| !title.is_empty()) {
            svg.push_str("<title>");
            svg.push_str(&escape_title(title));
            svg.push_str("</title>");
        }

        if let Some(path) = backdrop_path(options.background.as_ref()) {
            svg.push_str("<path d=\"");
            svg.push_str(&path.to_svg());
            svg.push_str("\" fill=\"");
            push_escaped_attribute_value(&mut svg, self.palette.bg.as_deref().unwrap_or_default());
            svg.push_str("\"/>");
        }

        let head = self.palette.head.as_deref().unwrap_or_default();
        let eye = self.palette.eye.as_deref().unwrap_or_default();
        if self.body_offset_y != 0.0 {
            svg.push_str(&format!(
                "<g transform=\"translate(0 {})\">",
                self.body_offset_y
            ));
        }
        svg.push_str("<g fill=\"");
        push_escaped_attribute_value(&mut svg, head);
        svg.push_str("\">");
        for petal in &self.layout.petals {
            svg.push_str("<circle cx=\"");
            svg.push_str(&rounded_number(petal.cx));
            svg.push_str("\" cy=\"");
            svg.push_str(&rounded_number(petal.cy));
            svg.push_str("\" r=\"");
            svg.push_str(&rounded_number(petal.r));
            svg.push_str("\"/>");
        }
        for path in self.layout.extra_paths() {
            svg.push_str("<path d=\"");
            svg.push_str(&path.to_svg());
            svg.push_str("\"/>");
        }
        svg.push_str("<path d=\"");
        svg.push_str(&self.layout.body_path().to_svg());
        svg.push_str("\"/></g><g fill=\"");
        push_escaped_attribute_value(&mut svg, eye);
        svg.push_str("\">");
        for path in self.layout.eye_paths() {
            svg.push_str("<path d=\"");
            svg.push_str(&path.to_svg());
            svg.push_str("\"/>");
        }
        svg.push_str("</g>");
        if self.body_offset_y != 0.0 {
            svg.push_str("</g>");
        }
        svg.push_str("</svg>");
        svg
    }

    pub fn uri(&self, options: &Options) -> String {
        let svg = self.svg(options).replace('"', "'");
        let mut escaped = String::with_capacity(svg.len());
        for character in svg.chars() {
            if matches!(
                character,
                '%' | '#' | '<' | '>' | '{' | '}' | '|' | '\\' | '^' | '[' | ']' | '`'
            ) {
                write_percent_encoded(&mut escaped, character);
            } else {
                escaped.push(character);
            }
        }

        let mut collapsed = String::with_capacity(escaped.len());
        let mut in_whitespace = false;
        for character in escaped.chars() {
            if is_ecmascript_whitespace(character) {
                if !in_whitespace {
                    collapsed.push(' ');
                }
                in_whitespace = true;
            } else {
                collapsed.push(character);
                in_whitespace = false;
            }
        }
        format!("data:image/svg+xml,{collapsed}")
    }
}

fn write_percent_encoded(output: &mut String, character: char) {
    output.push('%');
    output.push_str(&format!("{:02X}", character as u32));
}

fn escape_title(title: &str) -> String {
    title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn push_escaped_attribute_value(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
}

pub fn backdrop_path(background: Option<&Background>) -> Option<shape::Path> {
    match background {
        None | Some(Background::Enabled(false)) => None,
        Some(Background::Kind(kind)) if kind == "square" => Some(shape::Path::new(vec![
            shape::Command::MoveTo { x: 0.0, y: 0.0 },
            shape::Command::HorizontalTo { x: 100.0 },
            shape::Command::VerticalTo { y: 100.0 },
            shape::Command::HorizontalTo { x: 0.0 },
            shape::Command::Close,
        ])),
        Some(Background::Kind(kind)) if kind == "circle" => {
            Some(shape::superellipse(50.0, 50.0, 50.0, 50.0, 2.0, 0.0))
        }
        _ => Some(shape::superellipse(50.0, 50.0, 50.0, 50.0, 6.0, 0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Avatar, Background, Options, escape_title};
    use crate::color::Palette;
    use crate::shape::{Command, Path};

    #[test]
    fn rounds_negative_half_ties_toward_positive_infinity() {
        let path = Path::new(vec![
            Command::MoveTo {
                x: -1.125,
                y: -0.005,
            },
            Command::Close,
        ]);
        assert_eq!(path.to_svg(), "M-1.12 0Z");
    }

    #[test]
    fn does_not_round_a_value_below_half_up() {
        let value = 0.49999999999999994 / 100.0;
        assert_eq!(crate::shape::rounded(value), 0.0);
    }

    #[test]
    fn escapes_only_the_title_characters_from_the_static_core() {
        assert_eq!(escape_title("<&>\"'"), "&lt;&amp;&gt;\"'");
    }

    #[test]
    fn escapes_palette_values_before_svg_attribute_serialization() {
        let injected = r#"red" onload='alert(1)'&<script>"#;
        let options = Options {
            background: Some(Background::Enabled(true)),
            palette: Some(Palette {
                bg: Some(injected.to_string()),
                head: Some(injected.to_string()),
                eye: Some(injected.to_string()),
            }),
            ..Options::default()
        };
        let avatar = Avatar::new("unsafe", &options);
        let svg = avatar.svg(&options);
        let escaped = r#"red&quot; onload=&apos;alert(1)&apos;&amp;&lt;script&gt;"#;

        assert_eq!(avatar.palette.bg.as_deref(), Some(injected));
        assert_eq!(avatar.palette.head.as_deref(), Some(injected));
        assert_eq!(avatar.palette.eye.as_deref(), Some(injected));
        assert_eq!(svg.matches(&format!("fill=\"{escaped}\"")).count(), 3);
        assert!(!svg.contains(r#"fill="red" onload"#));
    }
}
