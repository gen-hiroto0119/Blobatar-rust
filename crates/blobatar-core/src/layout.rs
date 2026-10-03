use serde::{Serialize, Serializer};

use crate::{
    shape::{self, Path},
    traits::{TraitOverrides, Traits},
};

#[derive(Clone, Debug, Serialize)]
pub struct Body {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub n: f64,
    pub rot: f64,
    pub radii: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sides: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ellipse {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Face {
    Body(Body),
    Ellipse(Ellipse),
}

#[derive(Clone, Debug, Serialize)]
pub struct Eye {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    pub n: f64,
    pub rot: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Petal {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub shape: String,
    pub body: Body,
    pub face: Face,
    pub eyes: Vec<Eye>,
    pub petals: Vec<Petal>,
    extra: Vec<Path>,
    #[serde(skip)]
    body_path: Path,
}

impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_svg())
    }
}

impl Layout {
    pub fn body_path(&self) -> Path {
        self.body_path.clone()
    }

    pub fn eye_paths(&self) -> Vec<Path> {
        self.eyes
            .iter()
            .map(|eye| shape::superellipse(eye.cx, eye.cy, eye.rx, eye.ry, eye.n, eye.rot))
            .collect()
    }

    pub fn extra_svg(&self) -> Vec<String> {
        self.extra.iter().map(Path::to_svg).collect()
    }

    pub fn extra_paths(&self) -> &[Path] {
        &self.extra
    }
}

fn shape_name(value: f64) -> &'static str {
    if value < 0.22 {
        "round"
    } else if value < 0.48 {
        "organic"
    } else if value < 0.6 {
        "boxy"
    } else if value < 0.7 {
        "capsule"
    } else if value < 0.79 {
        "nub"
    } else if value < 0.86 {
        "cloud"
    } else if value < 0.915 {
        "droplet"
    } else if value < 0.95 {
        "hexagon"
    } else if value < 0.98 {
        "sun"
    } else {
        "triangle"
    }
}

fn body_for(shape_name: &str, traits: &Traits<'_>) -> Body {
    let core = match shape_name {
        "organic" => 0.98,
        "boxy" => 0.86,
        "capsule" => 1.02,
        "nub" => 0.88,
        "cloud" | "droplet" => 0.78,
        "hexagon" => 1.05,
        "sun" => 0.7,
        "triangle" => 1.15,
        _ => 1.0,
    };
    let radius = traits.num("body.r", 31.0, 38.0) * core;
    let mut radii = Vec::new();
    let count = traits.int("body.pts", 6, 8);
    for index in 0..count {
        radii.push(1.0 + traits.jitter(&format!("body.r{index}"), 0.16));
    }

    let mut body = Body {
        cx: 50.0 + traits.jitter("body.x", 1.5),
        cy: 50.0 + traits.jitter("body.y", 1.5),
        rx: radius,
        ry: radius * traits.num("body.ratio", 0.92, 1.08),
        n: traits.num("body.n", 1.9, 2.5),
        rot: 0.0,
        radii,
        sides: None,
        round: None,
    };

    match shape_name {
        "boxy" => {
            body.n = traits.num("body.n", 3.4, 6.0);
            body.rot = traits.num("body.rot", -20.0, 20.0);
        }
        "capsule" => body.ry *= traits.num("capsule.squat", 0.55, 0.68),
        "droplet" => {
            body.cy += 0.22 * body.ry;
            body.n = 2.0;
        }
        "hexagon" => {
            body.sides = Some(6);
            body.rot = traits.num("body.rot", -12.0, 12.0);
            body.round = Some(traits.num("poly.round", 0.24, 0.5));
        }
        "triangle" => {
            body.sides = Some(3);
            body.rot = traits.num("body.rot", -5.0, 5.0);
            body.round = Some(traits.num("poly.round", 0.24, 0.5));
        }
        _ => {}
    }
    body
}

fn face_for(shape_name: &str, body: &Body) -> Face {
    match shape_name {
        "organic" | "cloud" => {
            let scale = body.radii.iter().copied().fold(f64::INFINITY, f64::min) * 0.95;
            Face::Ellipse(Ellipse {
                cx: body.cx,
                cy: body.cy,
                rx: body.rx * scale,
                ry: body.ry * scale,
            })
        }
        "capsule" => Face::Ellipse(Ellipse {
            cx: body.cx,
            cy: body.cy,
            rx: body.rx * 0.94,
            ry: body.ry * 0.94,
        }),
        "droplet" => Face::Ellipse(Ellipse {
            cx: body.cx,
            cy: body.cy + body.ry * 0.05,
            rx: body.rx * 0.88,
            ry: body.ry * 0.88,
        }),
        "hexagon" => Face::Ellipse(Ellipse {
            cx: body.cx,
            cy: body.cy,
            rx: body.rx * 0.84,
            ry: body.ry * 0.84,
        }),
        "triangle" => Face::Ellipse(Ellipse {
            cx: body.cx,
            cy: body.cy + body.ry * 0.1,
            rx: body.rx * 0.54,
            ry: body.ry * 0.36,
        }),
        _ => Face::Body(body.clone()),
    }
}

fn face_ellipse(face: &Face) -> Ellipse {
    match face {
        Face::Body(body) => Ellipse {
            cx: body.cx,
            cy: body.cy,
            rx: body.rx,
            ry: body.ry,
        },
        Face::Ellipse(ellipse) => ellipse.clone(),
    }
}

fn eyes_for(traits: &Traits<'_>, body: &Body, face: &Ellipse) -> Vec<Eye> {
    let radius = body.rx;
    let base_radius = traits.num("eye.rx", 0.075, 0.105) * radius;
    let ratio = traits.num("eye.ratio", 1.9, 3.2);
    let scale = traits.num("eye.scale", 0.78, 1.24);
    let stretch = traits.num("eye.stretch", 0.85, 1.18);
    let clearance = traits.num("eye.gap", 0.1, 0.24) * radius;
    let wide = base_radius * 1.0_f64.max(scale);
    let tall = base_radius * ratio * 1.0_f64.max(scale * stretch);
    let base_gap = wide + radius * 0.03 + clearance;

    let gaze_x = traits.jitter("gaze.x", 0.09) * face.rx;
    let gaze_y = traits.num("gaze.y", -0.2, 0.08) * face.ry;
    let vertical_offset = traits.jitter("eye.dy", 0.04) * face.ry;
    let reach = wide.hypot(tall);
    let need = ((gaze_x.abs() + base_gap + reach) / face.rx)
        .hypot((gaze_y.abs() + vertical_offset.abs() + reach) / face.ry);
    let fit = if need > 0.9 { 0.9 / need } else { 1.0 };

    let eye_radius = base_radius * fit;
    let eye_ry = eye_radius * ratio;
    let gap = base_gap * fit;
    let room = (clearance / tall).clamp(0.0, 1.0);
    let bound = (room.asin() * 180.0 / std::f64::consts::PI).min(12.0);
    let lean = traits.num("eye.lean", -1.0, 1.0) * bound;
    let lean_right = (lean + traits.jitter("eye.lean2", 3.5)).clamp(-12.0, 12.0);
    let center_x = face.cx + gaze_x * fit;
    let center_y = face.cy + gaze_y * fit;
    let eye_n = traits.num("eye.n", 3.5, 6.0);

    vec![
        Eye {
            cx: center_x - gap,
            cy: center_y,
            rx: eye_radius,
            ry: eye_ry,
            n: eye_n,
            rot: lean,
        },
        Eye {
            cx: center_x + gap,
            cy: center_y + vertical_offset * fit,
            rx: eye_radius * scale,
            ry: eye_ry * scale * stretch,
            n: eye_n,
            rot: lean_right,
        },
    ]
}

fn petals_for(shape_name: &str, traits: &Traits<'_>, body: &Body) -> Vec<Petal> {
    match shape_name {
        "capsule" => [-1.0, 1.0]
            .into_iter()
            .map(|side| Petal {
                cx: body.cx + side * (body.rx - body.ry),
                cy: body.cy,
                r: body.ry,
            })
            .collect(),
        "nub" => {
            let count = traits.int("nub.n", 1, 2);
            (0..count)
                .map(|index| {
                    let key = format!("nub.a{index}");
                    let angle = traits.num(&key, 0.0, 2.0 * std::f64::consts::PI);
                    let radius_key = format!("nub.r{index}");
                    Petal {
                        cx: body.cx + angle.cos() * body.rx * 0.88,
                        cy: body.cy + angle.sin() * body.rx * 0.88,
                        r: body.rx * traits.num(&radius_key, 0.24, 0.4),
                    }
                })
                .collect()
        }
        "cloud" => {
            let count = traits.int("cloud.n", 4, 6);
            (0..count)
                .map(|index| {
                    let angle = std::f64::consts::PI
                        + std::f64::consts::PI * (index as f64 + 0.5) / count as f64;
                    let radius_key = format!("cloud.r{index}");
                    Petal {
                        cx: body.cx + angle.cos() * body.rx * 0.8,
                        cy: body.cy + angle.sin() * body.rx * 0.5,
                        r: body.rx * traits.num(&radius_key, 0.44, 0.62),
                    }
                })
                .collect()
        }
        "sun" => {
            let count = traits.int("sun.n", 6, 9);
            let distance = body.rx * traits.num("sun.dist", 1.0, 1.08);
            let radius = body.rx * traits.num("sun.r", 0.2, 0.26);
            let offset = traits.num("sun.rot", 0.0, 2.0 * std::f64::consts::PI);
            (0..count)
                .map(|index| {
                    let angle = offset + 2.0 * std::f64::consts::PI * index as f64 / count as f64;
                    Petal {
                        cx: body.cx + angle.cos() * distance,
                        cy: body.cy + angle.sin() * distance,
                        r: radius,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn extra_paths(shape_name: &str, traits: &Traits<'_>, body: &Body) -> Vec<Path> {
    if shape_name == "droplet" {
        vec![shape::taper(
            body.cx,
            body.cy,
            body.rx,
            body.ry,
            traits.num("droplet.tip", 1.4, 1.65),
        )]
    } else {
        Vec::new()
    }
}

fn body_path(shape_name: &str, body: &Body) -> Path {
    match shape_name {
        "organic" | "cloud" => {
            shape::blob_path(body.cx, body.cy, body.rx, body.ry, &body.radii, body.rot)
        }
        "capsule" => shape::box_path(body.cx, body.cy, body.rx - body.ry, body.ry),
        "hexagon" | "triangle" => shape::polygon(
            body.cx,
            body.cy,
            body.rx,
            body.ry,
            body.sides.unwrap(),
            body.round.unwrap(),
            body.rot,
        ),
        _ => shape::superellipse(body.cx, body.cy, body.rx, body.ry, body.n, body.rot),
    }
}

pub(crate) fn make_layout(seed: &str, normalize: bool, overrides: &TraitOverrides) -> Layout {
    let traits = Traits::new(seed, normalize, overrides);
    let shape_name = shape_name(traits.get("shape"));
    let body = body_for(shape_name, &traits);
    let face = face_for(shape_name, &body);
    let face_ellipse = face_ellipse(&face);
    Layout {
        shape: shape_name.to_owned(),
        eyes: eyes_for(&traits, &body, &face_ellipse),
        petals: petals_for(shape_name, &traits, &body),
        extra: extra_paths(shape_name, &traits, &body),
        body_path: body_path(shape_name, &body),
        body,
        face,
    }
}
