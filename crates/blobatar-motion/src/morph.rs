use blobatar_core::{Expression, Pose};
use serde::{Deserialize, Serialize};

use crate::ease;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fill {
    pub head: [u8; 3],
    pub eye: [u8; 3],
}

impl Fill {
    fn interpolate(self, to: Self, progress: f64) -> Self {
        let mix = |from: [u8; 3], to: [u8; 3]| {
            std::array::from_fn(|index| {
                let a = f64::from(from[index]);
                let b = f64::from(to[index]);
                (a + (b - a) * progress).round() as u8
            })
        };
        Self {
            head: mix(self.head, to.head),
            eye: mix(self.eye, to.eye),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct MorphFrame {
    pub pose: Pose,
    pub fill: Fill,
}

/// A persistent transition; the caller supplies one monotonic clock.
pub struct Morph {
    from: MorphFrame,
    to: MorphFrame,
    expression: Expression,
    start_ms: f64,
    duration_ms: f64,
}

impl Morph {
    pub fn new(expression: Expression, fill: Fill) -> Self {
        let frame = MorphFrame {
            pose: expression.pose(),
            fill,
        };
        Self {
            from: frame,
            to: frame,
            expression,
            start_ms: 0.0,
            duration_ms: 0.0,
        }
    }

    pub fn set_expression(&mut self, expression: Expression, fill: Fill, now_ms: f64) {
        let to = MorphFrame {
            pose: expression.pose(),
            fill,
        };
        if self.to == to {
            return;
        }
        self.from = self.sample(now_ms);
        self.to = to;
        self.expression = expression;
        self.start_ms = now_ms;
        self.duration_ms = if expression == Expression::Idle {
            400.0
        } else {
            300.0
        };
    }

    pub fn sample(&self, now_ms: f64) -> MorphFrame {
        if !self.is_running(now_ms) {
            return self.to;
        }
        let progress = ((now_ms - self.start_ms) / self.duration_ms).clamp(0.0, 1.0);
        let progress = if self.expression == Expression::Idle {
            ease::in_out(progress)
        } else {
            ease::morph_in(progress)
        };
        MorphFrame {
            pose: self.from.pose.interpolate(self.to.pose, progress),
            fill: self.from.fill.interpolate(self.to.fill, progress),
        }
    }

    pub fn is_running(&self, now_ms: f64) -> bool {
        self.duration_ms > 0.0 && now_ms < self.start_ms + self.duration_ms
    }

    pub fn finish(&mut self) {
        self.from = self.to;
        self.duration_ms = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILL: Fill = Fill {
        head: [255, 128, 0],
        eye: [0, 0, 0],
    };

    #[test]
    fn interrupted_expression_starts_from_current_frame_without_replaying() {
        let mut morph = Morph::new(Expression::Idle, FILL);
        morph.set_expression(Expression::Happy, FILL, 0.0);
        let before = morph.sample(125.0);
        morph.set_expression(Expression::Mad, FILL, 125.0);
        assert_eq!(morph.sample(125.0), before);
        morph.set_expression(Expression::Mad, FILL, 250.0);
        assert!(!morph.is_running(425.0));
        assert_eq!(morph.sample(425.0).pose, Expression::Mad.pose());
        morph.set_expression(Expression::Idle, FILL, 500.0);
        assert!(morph.is_running(899.0));
        assert_eq!(morph.sample(900.0).pose, Pose::IDENTITY);
        assert!(!morph.is_running(900.0));
    }

    #[test]
    fn reduced_motion_can_finish_without_another_frame_request() {
        let mut morph = Morph::new(Expression::Idle, FILL);
        let target = Fill {
            head: [10, 20, 30],
            eye: [255, 255, 255],
        };
        morph.set_expression(Expression::Love, target, 100.0);
        morph.finish();
        assert!(!morph.is_running(100.0));
        assert_eq!(morph.sample(100.0).fill, target);
        assert_eq!(morph.sample(100.0).pose, Expression::Love.pose());
    }
}
