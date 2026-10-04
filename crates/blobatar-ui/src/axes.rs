use blobatar_core::traits::Override;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Shape,
    Body,
    Eyes,
    Color,
    Decoration,
}

impl Group {
    pub const ALL: [Self; 5] = [
        Self::Shape,
        Self::Body,
        Self::Eyes,
        Self::Color,
        Self::Decoration,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Shape => "形状 / Shape",
            Self::Body => "体 / Body",
            Self::Eyes => "目 / Eyes",
            Self::Color => "配色 / Color",
            Self::Decoration => "装飾 / Decoration",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Slider,
    Shape,
    Tone,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Axis {
    pub key: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub kind: Kind,
    pub when: &'static [&'static str],
    pub bands: Option<usize>,
}

impl Axis {
    pub fn applies(&self, shapes: &[&str]) -> bool {
        self.when.is_empty() || shapes.iter().any(|shape| self.when.contains(shape))
    }
}

const fn slider(
    key: &'static str,
    label: &'static str,
    group: Group,
    when: &'static [&'static str],
    bands: Option<usize>,
) -> Axis {
    Axis {
        key,
        label,
        group,
        kind: Kind::Slider,
        when,
        bands,
    }
}

pub const AXES: [Axis; 25] = [
    Axis {
        key: "shape",
        label: "silhouette",
        group: Group::Shape,
        kind: Kind::Shape,
        when: &[],
        bands: None,
    },
    slider("body.r", "size", Group::Body, &[], None),
    slider("body.ratio", "proportion", Group::Body, &[], None),
    slider("body.n", "squareness", Group::Body, &[], None),
    slider(
        "body.rot",
        "tilt",
        Group::Body,
        &["boxy", "triangle", "hexagon"],
        None,
    ),
    slider("eye.rx", "size", Group::Eyes, &[], None),
    slider("eye.ratio", "roundness", Group::Eyes, &[], None),
    slider("eye.n", "squareness", Group::Eyes, &[], None),
    slider("eye.gap", "separation", Group::Eyes, &[], None),
    slider("eye.lean", "lean", Group::Eyes, &[], None),
    slider("gaze.x", "gaze x", Group::Eyes, &[], None),
    slider("gaze.y", "gaze y", Group::Eyes, &[], None),
    Axis {
        key: "tone",
        label: "tone",
        group: Group::Color,
        kind: Kind::Tone,
        when: &[],
        bands: None,
    },
    slider("hue", "hue", Group::Color, &[], None),
    slider("sun.n", "petals", Group::Decoration, &["sun"], Some(4)),
    slider(
        "sun.dist",
        "petal distance",
        Group::Decoration,
        &["sun"],
        None,
    ),
    slider("sun.r", "petal size", Group::Decoration, &["sun"], None),
    slider(
        "sun.rot",
        "petal rotation",
        Group::Decoration,
        &["sun"],
        None,
    ),
    slider("cloud.n", "lobes", Group::Decoration, &["cloud"], Some(3)),
    slider("nub.n", "nubs", Group::Decoration, &["nub"], Some(2)),
    slider("nub.a0", "nub angle", Group::Decoration, &["nub"], None),
    slider("nub.r0", "nub size", Group::Decoration, &["nub"], None),
    slider("capsule.squat", "squat", Group::Body, &["capsule"], None),
    slider(
        "poly.round",
        "corner rounding",
        Group::Body,
        &["triangle", "hexagon"],
        None,
    ),
    slider(
        "droplet.tip",
        "tip length",
        Group::Decoration,
        &["droplet"],
        None,
    ),
];

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Choice {
    pub name: &'static str,
    pub at: f64,
}

pub const SHAPES: [Choice; 10] = [
    Choice {
        name: "round",
        at: 0.11,
    },
    Choice {
        name: "organic",
        at: 0.35,
    },
    Choice {
        name: "boxy",
        at: 0.54,
    },
    Choice {
        name: "capsule",
        at: 0.65,
    },
    Choice {
        name: "nub",
        at: 0.745,
    },
    Choice {
        name: "cloud",
        at: 0.825,
    },
    Choice {
        name: "droplet",
        at: 0.888,
    },
    Choice {
        name: "hexagon",
        at: 0.933,
    },
    Choice {
        name: "sun",
        at: 0.965,
    },
    Choice {
        name: "triangle",
        at: 0.99,
    },
];
pub const GENERATION1_SHAPES: [Choice; 6] = [
    Choice {
        name: "round",
        at: 0.14,
    },
    Choice {
        name: "organic",
        at: 0.43,
    },
    Choice {
        name: "boxy",
        at: 0.65,
    },
    Choice {
        name: "nub",
        at: 0.78,
    },
    Choice {
        name: "cloud",
        at: 0.885,
    },
    Choice {
        name: "sun",
        at: 0.965,
    },
];
pub const TONES: [Choice; 6] = [
    Choice {
        name: "pastel",
        at: 0.1,
    },
    Choice {
        name: "pale",
        at: 0.28,
    },
    Choice {
        name: "mid",
        at: 0.49,
    },
    Choice {
        name: "deep",
        at: 0.71,
    },
    Choice {
        name: "bright",
        at: 0.865,
    },
    Choice {
        name: "ink",
        at: 0.965,
    },
];

pub fn toggle_choice(order: &[Choice], chosen: &[f64], at: f64) -> Vec<f64> {
    order
        .iter()
        .filter(|choice| {
            if choice.at == at {
                !chosen.contains(&at)
            } else {
                chosen.contains(&choice.at)
            }
        })
        .map(|choice| choice.at)
        .collect()
}

pub fn narrow_pin(values: Vec<f64>) -> Option<Override> {
    match values.as_slice() {
        [] => None,
        [value] => Some(Override::Fixed(*value)),
        _ => Some(Override::Candidates(values)),
    }
}

pub fn candidates<'a>(pin: Option<&Override>, resolved: &'a str) -> Vec<&'a str> {
    candidates_for(pin, resolved, &SHAPES)
}

pub fn candidates_for<'a>(
    pin: Option<&Override>,
    resolved: &'a str,
    shapes: &'static [Choice],
) -> Vec<&'a str> {
    let named = match pin {
        Some(Override::Candidates(values)) => values
            .iter()
            .filter_map(|value| {
                shapes
                    .iter()
                    .find(|shape| shape.at == *value)
                    .map(|shape| shape.name)
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    if named.is_empty() {
        vec![resolved]
    } else {
        named
    }
}

pub fn round3(value: f64) -> f64 {
    (value * 1000.0 + 0.5).floor() / 1000.0
}
pub fn band_value(index: usize, bands: usize) -> f64 {
    round3((index as f64 + 0.5) / bands as f64)
}
pub fn band_index(value: f64, bands: usize) -> usize {
    ((value * bands as f64).floor() as usize).min(bands.saturating_sub(1))
}
