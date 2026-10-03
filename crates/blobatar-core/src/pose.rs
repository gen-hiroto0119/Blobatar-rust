use serde::{Deserialize, Serialize};

/// Geometry channels match the upstream Pose; readable Rust names expose their role.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    #[serde(rename = "esx")]
    pub eye_scale_x: f64,
    #[serde(rename = "esy")]
    pub eye_scale_y: f64,
    #[serde(rename = "tilt")]
    pub eye_tilt: f64,
    #[serde(rename = "edy")]
    pub eye_offset_y: f64,
    #[serde(rename = "edx")]
    pub eye_offset_x: f64,
    #[serde(rename = "esx2")]
    pub right_scale_x_delta: f64,
    #[serde(rename = "esy2")]
    pub right_scale_y_delta: f64,
    #[serde(rename = "tilt2")]
    pub right_tilt_delta: f64,
    #[serde(rename = "edy2")]
    pub right_offset_y_delta: f64,
    #[serde(rename = "lock")]
    pub lean_lock: f64,
    pub heat: f64,
    pub shake: f64,
    pub rock: f64,
    #[serde(rename = "bdy")]
    pub body_offset_y: f64,
}

impl Pose {
    pub const IDENTITY: Self = Self {
        eye_scale_x: 1.0,
        eye_scale_y: 1.0,
        eye_tilt: 0.0,
        eye_offset_y: 0.0,
        eye_offset_x: 0.0,
        right_scale_x_delta: 0.0,
        right_scale_y_delta: 0.0,
        right_tilt_delta: 0.0,
        right_offset_y_delta: 0.0,
        lean_lock: 0.0,
        heat: 0.0,
        shake: 0.0,
        rock: 0.0,
        body_offset_y: 0.0,
    };

    pub fn interpolate(self, to: Self, progress: f64) -> Self {
        // Keep the upstream operation order rather than using a fused multiply-add.
        let mix = |a: f64, b: f64| a * (1.0 - progress) + b * progress;
        Self {
            eye_scale_x: mix(self.eye_scale_x, to.eye_scale_x),
            eye_scale_y: mix(self.eye_scale_y, to.eye_scale_y),
            eye_tilt: mix(self.eye_tilt, to.eye_tilt),
            eye_offset_y: mix(self.eye_offset_y, to.eye_offset_y),
            eye_offset_x: mix(self.eye_offset_x, to.eye_offset_x),
            right_scale_x_delta: mix(self.right_scale_x_delta, to.right_scale_x_delta),
            right_scale_y_delta: mix(self.right_scale_y_delta, to.right_scale_y_delta),
            right_tilt_delta: mix(self.right_tilt_delta, to.right_tilt_delta),
            right_offset_y_delta: mix(self.right_offset_y_delta, to.right_offset_y_delta),
            lean_lock: mix(self.lean_lock, to.lean_lock),
            heat: mix(self.heat, to.heat),
            shake: mix(self.shake, to.shake),
            rock: mix(self.rock, to.rock),
            body_offset_y: mix(self.body_offset_y, to.body_offset_y),
        }
    }
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Expression {
    #[default]
    Idle,
    Happy,
    Sad,
    Mad,
    Surprised,
    Wink,
    Sleepy,
    Smug,
    Unsure,
    Scared,
    Love,
    Shy,
    Sick,
    Thinking,
}

impl Expression {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Happy => "happy",
            Self::Sad => "sad",
            Self::Mad => "mad",
            Self::Surprised => "surprised",
            Self::Wink => "wink",
            Self::Sleepy => "sleepy",
            Self::Smug => "smug",
            Self::Unsure => "unsure",
            Self::Scared => "scared",
            Self::Love => "love",
            Self::Shy => "shy",
            Self::Sick => "sick",
            Self::Thinking => "thinking",
        }
    }

    pub fn palette(self, base: &crate::color::Palette) -> crate::color::Palette {
        use crate::color::{self, BILE, BLUSH, HOT, ROSE};
        let target = match self {
            Self::Mad => HOT,
            Self::Love => ROSE,
            Self::Shy => BLUSH,
            Self::Sick => BILE,
            _ => return base.clone(),
        };
        let mut palette = base.clone();
        if let (Some(head), Some(eye)) = (&base.head, &base.eye) {
            let (hot_head, hot_eye) = color::tinted(head, eye, target);
            palette.head = Some(color::mix_hex(head, &hot_head, self.pose().heat));
            palette.eye = Some(color::mix_hex(eye, &hot_eye, self.pose().heat));
        }
        palette
    }
    pub const ALL: [Self; 14] = [
        Self::Idle,
        Self::Happy,
        Self::Sad,
        Self::Mad,
        Self::Surprised,
        Self::Wink,
        Self::Sleepy,
        Self::Smug,
        Self::Unsure,
        Self::Scared,
        Self::Love,
        Self::Shy,
        Self::Sick,
        Self::Thinking,
    ];

    pub fn pose(self) -> Pose {
        match self {
            Self::Idle => Pose::IDENTITY,
            Self::Happy => Pose {
                eye_scale_x: 1.72,
                eye_scale_y: 0.3,
                eye_tilt: 8.0,
                eye_offset_y: -1.5,
                eye_offset_x: 1.5,
                right_scale_x_delta: 0.08,
                right_scale_y_delta: 0.05,
                right_tilt_delta: -16.0,
                lean_lock: 1.0,
                body_offset_y: -2.2,
                ..Pose::IDENTITY
            },
            Self::Sad => Pose {
                eye_scale_x: 0.6,
                eye_scale_y: 0.56,
                eye_tilt: 26.0,
                eye_offset_y: 3.6,
                eye_offset_x: 1.9,
                right_scale_x_delta: -0.05,
                right_scale_y_delta: -0.07,
                right_tilt_delta: -7.0,
                lean_lock: 1.0,
                body_offset_y: 2.6,
                ..Pose::IDENTITY
            },
            Self::Mad => Pose {
                eye_scale_x: 1.85,
                eye_scale_y: 0.26,
                eye_tilt: -33.0,
                eye_offset_y: 0.4,
                eye_offset_x: 0.6,
                right_scale_y_delta: -0.03,
                right_tilt_delta: 5.0,
                lean_lock: 1.0,
                heat: 0.62,
                shake: 0.55,
                body_offset_y: 0.8,
                ..Pose::IDENTITY
            },
            Self::Surprised => Pose {
                eye_scale_x: 1.34,
                eye_scale_y: 1.2,
                eye_tilt: -6.0,
                eye_offset_y: -1.05,
                eye_offset_x: 0.5,
                right_scale_x_delta: 0.05,
                right_scale_y_delta: 0.07,
                right_tilt_delta: 3.0,
                lean_lock: 1.0,
                body_offset_y: -1.4,
                ..Pose::IDENTITY
            },
            Self::Wink => Pose {
                eye_scale_x: 1.32,
                eye_scale_y: 0.76,
                eye_tilt: 5.0,
                eye_offset_y: -0.6,
                eye_offset_x: 0.8,
                right_scale_x_delta: 0.26,
                right_scale_y_delta: -0.56,
                right_tilt_delta: -11.0,
                lean_lock: 1.0,
                body_offset_y: -1.1,
                ..Pose::IDENTITY
            },
            Self::Sleepy => Pose {
                eye_scale_x: 1.14,
                eye_scale_y: 0.22,
                eye_offset_y: 2.4,
                eye_offset_x: 0.3,
                right_scale_x_delta: -0.04,
                right_scale_y_delta: 0.03,
                right_tilt_delta: 4.0,
                lean_lock: 1.0,
                body_offset_y: 1.2,
                ..Pose::IDENTITY
            },
            Self::Smug => Pose {
                eye_scale_x: 1.3,
                eye_scale_y: 0.42,
                eye_tilt: 18.0,
                eye_offset_y: -0.5,
                eye_offset_x: 0.5,
                right_scale_x_delta: 0.06,
                right_scale_y_delta: -0.06,
                right_tilt_delta: -36.0,
                lean_lock: 1.0,
                body_offset_y: -1.0,
                ..Pose::IDENTITY
            },
            Self::Unsure => Pose {
                eye_scale_x: 0.95,
                eye_scale_y: 1.02,
                eye_tilt: 4.0,
                eye_offset_y: -0.2,
                eye_offset_x: 0.3,
                right_scale_x_delta: 0.24,
                right_scale_y_delta: -0.44,
                right_tilt_delta: -18.0,
                lean_lock: 1.0,
                ..Pose::IDENTITY
            },
            Self::Scared => Pose {
                eye_scale_x: 0.78,
                eye_scale_y: 0.96,
                eye_tilt: -12.0,
                eye_offset_y: -1.5,
                eye_offset_x: -0.8,
                right_scale_x_delta: -0.04,
                right_scale_y_delta: 0.05,
                right_tilt_delta: 4.0,
                lean_lock: 1.0,
                shake: 0.35,
                body_offset_y: -0.6,
                ..Pose::IDENTITY
            },
            Self::Love => Pose {
                eye_scale_x: 0.86,
                eye_scale_y: 1.28,
                eye_tilt: -14.0,
                eye_offset_y: -0.5,
                eye_offset_x: -0.35,
                right_scale_x_delta: 0.05,
                right_scale_y_delta: 0.06,
                right_tilt_delta: 6.0,
                lean_lock: 1.0,
                heat: 0.6,
                body_offset_y: -1.6,
                ..Pose::IDENTITY
            },
            Self::Shy => Pose {
                eye_scale_x: 0.62,
                eye_scale_y: 0.5,
                eye_tilt: 10.0,
                eye_offset_y: 1.4,
                eye_offset_x: -0.2,
                right_scale_x_delta: -0.05,
                right_scale_y_delta: -0.04,
                right_tilt_delta: -8.0,
                lean_lock: 1.0,
                heat: 0.55,
                body_offset_y: 0.9,
                ..Pose::IDENTITY
            },
            Self::Sick => Pose {
                eye_scale_x: 1.25,
                eye_scale_y: 0.34,
                eye_tilt: 20.0,
                eye_offset_y: 1.8,
                eye_offset_x: 0.8,
                right_scale_x_delta: 0.05,
                right_scale_y_delta: -0.05,
                right_tilt_delta: -6.0,
                lean_lock: 1.0,
                heat: 0.6,
                shake: 0.18,
                body_offset_y: 1.4,
                ..Pose::IDENTITY
            },
            Self::Thinking => Pose {
                eye_scale_x: 1.15,
                eye_scale_y: 0.62,
                eye_offset_y: 4.2,
                eye_offset_x: 0.4,
                right_scale_x_delta: 0.02,
                right_scale_y_delta: 0.06,
                right_offset_y_delta: -8.4,
                lean_lock: 1.0,
                rock: 0.8,
                body_offset_y: -0.4,
                ..Pose::IDENTITY
            },
        }
    }
}

pub fn bake(layout: &crate::Layout, pose: Pose) -> crate::Layout {
    let mut result = layout.clone();
    for (index, eye) in result.eyes.iter_mut().enumerate() {
        let right = index != 0;
        let side = if right { 1.0 } else { -1.0 };
        eye.cx += pose.eye_offset_x * side;
        eye.cy += pose.eye_offset_y
            + if right {
                pose.right_offset_y_delta
            } else {
                0.0
            };
        eye.rx *= pose.eye_scale_x + if right { pose.right_scale_x_delta } else { 0.0 };
        eye.ry *= pose.eye_scale_y + if right { pose.right_scale_y_delta } else { 0.0 };
        eye.rot = eye.rot * (1.0 - pose.lean_lock)
            + (pose.eye_tilt + if right { pose.right_tilt_delta } else { 0.0 }) * side;
    }
    result
}
