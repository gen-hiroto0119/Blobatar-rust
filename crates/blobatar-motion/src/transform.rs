use blobatar_core::{Layout, Pose};
use serde::{Deserialize, Serialize};

use crate::{gaze::Projection, idle::IdleFrame};

/// SVG affine order: [a, b, c, d, translation_x, translation_y].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Affine(pub [f64; 6]);

impl Affine {
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translate(x: f64, y: f64) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, x, y])
    }

    pub fn scale(x: f64, y: f64) -> Self {
        Self([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    pub fn rotate(degrees: f64) -> Self {
        let radians = degrees * std::f64::consts::PI / 180.0;
        let (sin, cos) = radians.sin_cos();
        Self([cos, sin, -sin, cos, 0.0, 0.0])
    }

    /// Apply `inner` first, then this transform (SVG transform-list order).
    pub fn compose(self, inner: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = inner.0;
        Self([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ])
    }

    pub fn apply(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
}

fn round3(value: f64) -> f64 {
    let scaled = value * 1000.0;
    let floor = scaled.floor();
    (floor + if scaled - floor >= 0.5 { 1.0 } else { 0.0 }) / 1000.0
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameTransforms {
    pub body: Affine,
    pub eyes: Vec<Affine>,
}

/// Transform cached, neutral paths without regenerating geometry or palettes.
pub fn frame_transforms(layout: &Layout, pose: Pose, idle: IdleFrame) -> FrameTransforms {
    frame_transforms_with_hover(layout, pose, idle, Affine::IDENTITY)
}

pub fn frame_transforms_with_hover(
    layout: &Layout,
    pose: Pose,
    idle: IdleFrame,
    hover: Affine,
) -> FrameTransforms {
    frame_transforms_with_gaze(layout, pose, idle, hover, &[])
}

/// Fold gaze into the pose's unscaled offset, tilt and local eye scales.
pub fn frame_transforms_with_gaze(
    layout: &Layout,
    pose: Pose,
    idle: IdleFrame,
    hover: Affine,
    gaze: &[Projection],
) -> FrameTransforms {
    let body = Affine::translate(round3(idle.shake[0]), round3(idle.shake[1]))
        .compose(hover)
        .compose(Affine::translate(50.0, 50.0))
        .compose(Affine::scale(
            round3(idle.breathe[0]),
            round3(idle.breathe[1]),
        ))
        .compose(Affine::translate(-50.0, -50.0))
        .compose(Affine::translate(
            0.0,
            round3(pose.body_offset_y + idle.bob),
        ));
    let pair = body.compose(Affine::translate(
        round3(idle.saccade[0]),
        round3(idle.saccade[1]),
    ));
    let eyes = layout
        .eyes
        .iter()
        .enumerate()
        .map(|(index, eye)| {
            let gaze = gaze.get(index).copied().unwrap_or(Projection::IDENTITY);
            let select = if index == 0 { 0.0 } else { 1.0 };
            let side = if index == 0 { -1.0 } else { 1.0 };
            let phase = select * (1.0 - pose.rock) + pose.rock * ((1.0 + side * idle.rockp) / 2.0);
            let posed = Affine::translate(
                round3(eye.cx + pose.eye_offset_x * side) + gaze.dx,
                round3(eye.cy + pose.eye_offset_y + phase * pose.right_offset_y_delta) + gaze.dy,
            )
            .compose(Affine::rotate(
                round3(
                    (pose.eye_tilt + select * pose.right_tilt_delta) * side
                        + eye.rot * (1.0 - pose.lean_lock),
                ) + gaze.tilt,
            ))
            .compose(Affine::scale(
                round3(pose.eye_scale_x + select * pose.right_scale_x_delta) * gaze.sx,
                round3(pose.eye_scale_y + select * pose.right_scale_y_delta) * gaze.sy,
            ))
            .compose(Affine::rotate(round3(-eye.rot)))
            .compose(Affine::translate(round3(-eye.cx), round3(-eye.cy)));
            let glance = Affine::translate(round3(eye.cx), round3(eye.cy))
                .compose(Affine::rotate(round3(idle.wrap.rot * side)))
                .compose(Affine::scale(
                    round3(1.0 + idle.wrap.mx + idle.wrap.side * side),
                    round3(1.0 + idle.wrap.sy),
                ))
                .compose(Affine::rotate(round3(eye.rot)))
                .compose(Affine::scale(1.0, round3(idle.blink)))
                .compose(Affine::rotate(round3(-eye.rot)))
                .compose(Affine::translate(round3(-eye.cx), round3(-eye.cy)));
            pair.compose(posed).compose(glance)
        })
        .collect();
    FrameTransforms { body, eyes }
}
