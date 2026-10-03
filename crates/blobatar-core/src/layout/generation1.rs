use super::{Eye, Face, Layout, body_for, body_path, petals_for};
use crate::traits::{TraitOverrides, Traits};

fn shape_name(value: f64) -> &'static str {
    if value < 0.28 {
        "round"
    } else if value < 0.58 {
        "organic"
    } else if value < 0.72 {
        "boxy"
    } else if value < 0.84 {
        "nub"
    } else if value < 0.93 {
        "cloud"
    } else {
        "sun"
    }
}

pub(crate) fn make_layout(seed: &str, normalize: bool, overrides: &TraitOverrides) -> Layout {
    let traits = Traits::new(seed, normalize, overrides);
    let shape = shape_name(traits.get("shape"));
    let body = body_for(shape, &traits);
    let gaze_x = traits.jitter("gaze.x", 0.09) * body.rx;
    let gaze_y = traits.num("gaze.y", -0.2, 0.08) * body.ry;
    let base_radius = traits.num("eye.rx", 0.075, 0.105) * body.rx;
    let ratio = traits.num("eye.ratio", 1.9, 3.2);
    let scale = traits.num("eye.scale", 0.78, 1.24);
    let stretch = traits.num("eye.stretch", 0.85, 1.18);
    let clearance = traits.num("eye.gap", 0.1, 0.24) * body.rx;
    let wide = base_radius * scale.max(1.0);
    let tall = base_radius * ratio * (scale * stretch).max(1.0);
    let base_gap = wide + body.rx * 0.03 + clearance;
    let tight = if matches!(shape, "organic" | "cloud") {
        body.radii.iter().copied().fold(f64::INFINITY, f64::min) * 0.95
    } else {
        1.0
    };
    // Generation 1 fits horizontally and leaves gaze offsets unscaled.
    let need = (gaze_x.abs() + base_gap + wide.hypot(tall)) / body.rx;
    let fit = if need > tight * 0.9 {
        tight * 0.9 / need
    } else {
        1.0
    };
    let radius = base_radius * fit;
    let gap = base_gap * fit;
    let eye_ry = radius * ratio;
    let room = ((clearance * fit) / (tall * fit)).clamp(0.0, 1.0);
    let bound = (room.asin() * 180.0 / std::f64::consts::PI).min(12.0);
    let lean = traits.num("eye.lean", -1.0, 1.0) * bound;
    let lean_right = (lean + traits.jitter("eye.lean2", 3.5)).clamp(-12.0, 12.0);
    let eye_n = traits.num("eye.n", 3.5, 6.0);
    let eyes = vec![
        Eye {
            cx: body.cx + gaze_x - gap,
            cy: body.cy + gaze_y,
            rx: radius,
            ry: eye_ry,
            n: eye_n,
            rot: lean,
        },
        Eye {
            cx: body.cx + gaze_x + gap,
            cy: body.cy + gaze_y + traits.jitter("eye.dy", 0.04) * body.ry,
            rx: radius * scale,
            ry: eye_ry * scale * stretch,
            n: eye_n,
            rot: lean_right,
        },
    ];
    Layout {
        shape: shape.into(),
        eyes,
        petals: petals_for(shape, &traits, &body),
        extra: Vec::new(),
        body_path: body_path(shape, &body),
        face: Face::Body(body.clone()),
        body,
    }
}
