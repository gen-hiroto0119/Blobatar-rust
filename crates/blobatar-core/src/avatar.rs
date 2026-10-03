use serde::{Deserialize, Serialize};

use crate::{
    color::{self, Palette},
    hash::is_ecmascript_whitespace,
    layout::{self, Layout},
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
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Avatar {
    pub layout: Layout,
    pub palette: Palette,
}

impl Avatar {
    pub fn new(seed: &str, options: &Options) -> Self {
        let layout = layout::make_layout(seed, options.normalize, &options.traits);
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

        Self { layout, palette }
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
            svg.push_str(self.palette.bg.as_deref().unwrap_or_default());
            svg.push_str("\"/>");
        }

        let head = self.palette.head.as_deref().unwrap_or_default();
        let eye = self.palette.eye.as_deref().unwrap_or_default();
        svg.push_str("<g fill=\"");
        svg.push_str(head);
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
        svg.push_str(eye);
        svg.push_str("\">");
        for path in self.layout.eye_paths() {
            svg.push_str("<path d=\"");
            svg.push_str(&path.to_svg());
            svg.push_str("\"/>");
        }
        svg.push_str("</g></svg>");
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
    use super::escape_title;
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
}
