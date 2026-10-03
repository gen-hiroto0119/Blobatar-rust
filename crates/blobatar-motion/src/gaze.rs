use serde::{Deserialize, Serialize};

pub const SETTLE_MS: f64 = 110.0;
pub const SNAP: f64 = 1.6;
pub const HOLD_EPSILON: f64 = 0.01;
const DEADZONE: f64 = 0.55;
const LIMB: f64 = 0.97;

pub fn pursuit(delta_ms: f64, settle_ms: f64) -> f64 {
    if settle_ms <= 0.0 {
        1.0
    } else {
        1.0 - (-delta_ms / settle_ms).exp()
    }
}

pub fn threshold(width: f64, travel: f64) -> f64 {
    let per_unit = travel * (width / 100.0);
    if per_unit > 0.0 {
        (0.15 / per_unit).clamp(0.002, 0.06)
    } else {
        0.06
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Mark {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Projection {
    pub dx: f64,
    pub dy: f64,
    pub sx: f64,
    pub sy: f64,
    #[serde(rename = "t")]
    pub tilt: f64,
}

struct Patch {
    scale_x: f64,
    scale_y: f64,
    shear: f64,
}

fn patch(x: f64, y: f64, z: f64) -> Patch {
    let radius_squared = x * x + y * y;
    if radius_squared < 1e-9 {
        return Patch {
            scale_x: 1.0,
            scale_y: 1.0,
            shear: 0.0,
        };
    }
    let depth = z.max(0.0);
    Patch {
        scale_x: (depth * x * x + y * y) / radius_squared,
        scale_y: (depth * y * y + x * x) / radius_squared,
        shear: ((depth - 1.0) * x * y) / radius_squared,
    }
}

pub fn project(mark: Mark, yaw: f64, pitch: f64) -> Projection {
    let radius_squared = mark.x * mark.x + mark.y * mark.y;
    let scale = if radius_squared > 1.0 {
        1.0 / radius_squared.sqrt()
    } else {
        1.0
    };
    let x0 = mark.x * scale;
    let y0 = mark.y * scale;
    let z0 = (1.0 - x0 * x0 - y0 * y0).max(0.0).sqrt();
    let x1 = x0 * yaw.cos() + z0 * yaw.sin();
    let z1 = z0 * yaw.cos() - x0 * yaw.sin();
    let y1 = y0 * pitch.cos() + z1 * pitch.sin();
    let z2 = z1 * pitch.cos() - y0 * pitch.sin();
    let radius = x1.hypot(y1);
    let over_limb = z2 <= 0.0 || radius > LIMB;
    let back = if over_limb {
        LIMB / if radius != 0.0 { radius } else { 1.0 }
    } else {
        1.0
    };
    let px = x1 * back;
    let py = y1 * back;
    let pz = if over_limb {
        (1.0 - LIMB * LIMB).sqrt()
    } else {
        z2
    };
    let rest = patch(x0, y0, z0);
    let now = patch(px, py, pz);
    Projection {
        dx: px - mark.x,
        dy: py - mark.y,
        sx: (now.scale_x
            / if rest.scale_x != 0.0 {
                rest.scale_x
            } else {
                1.0
            })
        .min(1.0),
        sy: (now.scale_y
            / if rest.scale_y != 0.0 {
                rest.scale_y
            } else {
                1.0
            })
        .min(1.0),
        tilt: 4.0 * (now.shear - rest.shear),
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StepInput {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub radius: f64,
    #[serde(rename = "k")]
    pub pursuit: f64,
    pub gain: f64,
    pub snap: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StepResult {
    pub x: f64,
    pub y: f64,
    pub tx: f64,
    pub ty: f64,
    #[serde(rename = "f")]
    pub factor: f64,
}

pub fn step(input: StepInput) -> StepResult {
    let distance = input.dx.hypot(input.dy);
    let near = if input.radius > 0.0 {
        let t = (distance / (input.radius * DEADZONE)).min(1.0);
        t * t * (3.0 - 2.0 * t)
    } else {
        1.0
    };
    let amplitude = input.gain * near;
    let tx = if distance > 0.0 {
        (input.dx / distance) * amplitude
    } else {
        0.0
    };
    let ty = if distance > 0.0 {
        (input.dy / distance) * amplitude
    } else {
        0.0
    };
    let factor = if (tx - input.x).hypot(ty - input.y) > input.snap {
        1.0
    } else {
        input.pursuit
    };
    StepResult {
        x: input.x + (tx - input.x) * factor,
        y: input.y + (ty - input.y) * factor,
        tx,
        ty,
        factor,
    }
}
