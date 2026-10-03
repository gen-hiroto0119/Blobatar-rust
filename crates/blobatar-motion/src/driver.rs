use blobatar_core::geometry::Bounds;
use serde::Serialize;

use crate::{
    gaze::{self, HOLD_EPSILON, Mark, Projection, SETTLE_MS, SNAP, StepInput},
    survey::Face,
};

#[derive(Clone, Copy, Debug, Default)]
pub enum Target {
    #[default]
    None,
    Rest,
    Pointer,
    Point(Mark),
    /// The host retains the element identity and supplies its latest window bounds.
    Element(Bounds),
}

#[derive(Clone, Debug, Serialize)]
pub struct GazeFrame {
    pub direction: Mark,
    pub hold: f64,
    pub eyes: Vec<Projection>,
}

impl GazeFrame {
    fn neutral(eyes: usize) -> Self {
        Self {
            direction: Mark::default(),
            hold: 0.0,
            eyes: vec![
                Projection {
                    dx: 0.0,
                    dy: 0.0,
                    sx: 1.0,
                    sy: 1.0,
                    tilt: 0.0
                };
                eyes
            ],
        }
    }
}

/// Event-driven pursuit. The host supplies time, layout and pointer events.
pub struct GazeDriver {
    face: Face,
    bounds: Bounds,
    travel: f64,
    target: Target,
    pointer: Mark,
    position: Mark,
    written: Mark,
    hold: f64,
    written_hold: f64,
    frame: GazeFrame,
    last_ms: f64,
    scheduled: bool,
    dirty: bool,
    enabled: bool,
    stopped: bool,
    settle_ms: f64,
    snap: f64,
}

impl GazeDriver {
    pub fn new(face: Face, bounds: Bounds, travel: f64) -> Self {
        Self {
            frame: GazeFrame::neutral(face.marks.len()),
            face,
            bounds,
            travel,
            target: Target::None,
            pointer: Mark { x: -1e6, y: -1e6 },
            position: Mark::default(),
            written: Mark::default(),
            hold: 0.0,
            written_hold: 0.0,
            last_ms: 0.0,
            scheduled: true,
            dirty: true,
            enabled: true,
            stopped: false,
            settle_ms: SETTLE_MS,
            snap: SNAP,
        }
    }

    pub fn tuning(mut self, settle_ms: f64, snap: f64) -> Self {
        self.settle_ms = settle_ms;
        self.snap = snap;
        self
    }

    pub fn look_at(&mut self, target: Target) {
        self.target = target;
        self.wake();
    }

    pub fn pointer_moved(&mut self, point: Mark) {
        if self.enabled {
            self.pointer = point;
            self.wake();
        }
    }

    pub fn pointer_left(&mut self) {
        // Preserve the pinned driver's sentinel, including its far upper-left aim.
        self.pointer_moved(Mark { x: -1e6, y: -1e6 });
    }

    pub fn remeasure(&mut self, bounds: Bounds, travel: f64, watched: Option<Bounds>) {
        self.bounds = bounds;
        self.travel = travel;
        if matches!(self.target, Target::Element(_))
            && let Some(bounds) = watched
        {
            self.target = Target::Element(bounds);
        }
        self.wake();
    }

    /// A missing survey keeps the last usable geometry, like an unlaid-out SVG.
    pub fn replace_face(&mut self, face: Option<Face>) {
        if let Some(face) = face {
            self.face = face;
        }
        self.frame.direction = Mark::default();
        self.frame.eyes = GazeFrame::neutral(self.face.marks.len()).eyes;
        self.written = Mark {
            x: f64::INFINITY,
            y: f64::INFINITY,
        };
        self.wake();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.stopped || enabled == self.enabled {
            return;
        }
        self.enabled = enabled;
        if enabled {
            self.wake();
        } else {
            self.position = Mark::default();
            self.written = Mark::default();
            self.hold = 0.0;
            self.written_hold = 0.0;
            self.frame = GazeFrame::neutral(self.face.marks.len());
            self.last_ms = 0.0;
            self.scheduled = false;
        }
    }

    /// Teardown is immediate and permanent; `Target::None` instead eases home.
    pub fn stop(&mut self) {
        self.set_enabled(false);
        self.stopped = true;
    }

    fn wake(&mut self) {
        if self.enabled {
            self.dirty = true;
            self.scheduled = true;
        }
    }

    pub fn needs_frame(&self) -> bool {
        self.scheduled
    }

    pub fn frame(&self) -> &GazeFrame {
        &self.frame
    }

    pub fn tick(&mut self, time_ms: f64) {
        if !self.scheduled {
            return;
        }
        let dt = if self.last_ms != 0.0 {
            (time_ms - self.last_ms).min(64.0)
        } else {
            16.0
        };
        self.last_ms = time_ms;
        let k = gaze::pursuit(dt, self.settle_ms);
        let [cx, cy] = self.bounds.center();
        let point = match self.target {
            Target::None | Target::Rest => Mark { x: cx, y: cy },
            Target::Pointer => self.pointer,
            Target::Point(point) => point,
            Target::Element(bounds) => {
                let [x, y] = bounds.center();
                Mark { x, y }
            }
        };
        let result = gaze::step(StepInput {
            x: self.position.x,
            y: self.position.y,
            dx: point.x - cx,
            dy: point.y - cy,
            radius: (self.bounds.width.min(self.bounds.height) / 2.0).max(1.0),
            pursuit: k,
            gain: 1.0,
            snap: self.snap,
        });
        self.position = Mark {
            x: result.x,
            y: result.y,
        };
        let eps = gaze::threshold(self.bounds.width.max(1.0), self.travel);
        let mut moved = false;
        if (result.tx - result.x).abs() <= eps && (result.ty - result.y).abs() <= eps {
            self.position = Mark {
                x: result.tx,
                y: result.ty,
            };
        } else {
            moved = true;
        }
        let hold = if matches!(self.target, Target::None) {
            0.0
        } else {
            1.0
        };
        self.hold += (hold - self.hold) * k;
        if (hold - self.hold).abs() <= HOLD_EPSILON {
            self.hold = hold;
        } else {
            moved = true;
        }
        if (self.hold - self.written_hold).abs() > HOLD_EPSILON {
            self.written_hold = self.hold;
            self.frame.hold = fixed(self.hold, 3);
            moved = true;
        }
        if (self.position.x - self.written.x).abs() > eps
            || (self.position.y - self.written.y).abs() > eps
        {
            self.written = self.position;
            self.frame.direction = Mark {
                x: fixed(self.position.x, 3),
                y: fixed(self.position.y, 3),
            };
            self.frame.eyes = self
                .face
                .marks
                .iter()
                .map(|&mark| {
                    let p = gaze::project(
                        mark,
                        self.position.x * (self.travel / self.face.rx),
                        self.position.y * (self.travel / self.face.ry),
                    );
                    Projection {
                        dx: fixed(p.dx * self.face.rx, 3),
                        dy: fixed(p.dy * self.face.ry, 3),
                        sx: fixed(p.sx, 4),
                        sy: fixed(p.sy, 4),
                        tilt: fixed(p.tilt, 3),
                    }
                })
                .collect();
            moved = true;
        }
        self.scheduled = moved || self.dirty;
        self.dirty = false;
        if !self.scheduled {
            self.last_ms = 0.0;
        }
    }
}

// Numeric value of JS toFixed(0..4): round the exact binary fraction, ties away.
fn fixed(value: f64, digits: u32) -> f64 {
    assert!(digits <= 4);
    if !value.is_finite() || value.abs() >= 1e21 || value == 0.0 {
        return value;
    }
    let bits = value.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1_u64 << 52) - 1)) | if exponent != 0 { 1_u64 << 52 } else { 0 };
    let numerator = u128::from(mantissa) * 5_u128.pow(digits);
    let power = exponent.max(1) - 1023 - 52 + digits as i32;
    let rounded = if power >= 0 {
        numerator << power
    } else if power <= -128 {
        0
    } else {
        let shift = -power;
        let half = 1_u128 << (shift - 1);
        (numerator >> shift) + u128::from(numerator & ((half << 1) - 1) >= half)
    };
    (rounded as f64 / 10_f64.powi(digits as i32)).copysign(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn close(actual: &Value, expected: &Value) {
        match expected {
            Value::Number(value) => {
                let expected = value.as_f64().unwrap();
                let actual = actual.as_f64().unwrap();
                assert!(
                    (actual - expected).abs()
                        <= 1e-12_f64.max(1e-9 * actual.abs().max(expected.abs())),
                    "{actual} != {expected}"
                );
            }
            Value::Array(values) => {
                assert_eq!(actual.as_array().unwrap().len(), values.len());
                for (a, b) in actual.as_array().unwrap().iter().zip(values) {
                    close(a, b);
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    close(&actual[key], value);
                }
            }
            _ => assert_eq!(actual, expected),
        }
    }

    #[test]
    fn driver_event_sequences_match_pinned_upstream() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/driver-2.7.0.json")).unwrap();
        let survey: Value =
            serde_json::from_str(include_str!("../tests/fixtures/survey-2.7.0.json")).unwrap();
        assert_eq!(fixture["meta"]["caseCount"], 12);
        for case in fixture["cases"].as_array().unwrap() {
            let size = case["size"].as_f64().unwrap();
            let travel = case["travel"].as_f64().unwrap();
            let face = |index: &Value| {
                serde_json::from_value(
                    survey["cases"][index.as_u64().unwrap() as usize]["face"].clone(),
                )
                .unwrap()
            };
            let mut driver = GazeDriver::new(
                face(&case["index"]),
                Bounds {
                    x: 100.0,
                    y: 80.0,
                    width: size,
                    height: size,
                },
                travel,
            );
            let mut watched = Bounds {
                x: 360.0,
                y: 200.0,
                width: 80.0,
                height: 32.0,
            };
            for row in case["actions"].as_array().unwrap() {
                let action = &row["action"];
                match action["kind"].as_str().unwrap() {
                    "tick" => driver.tick(action["time"].as_f64().unwrap()),
                    "pointer" => driver.pointer_moved(Mark {
                        x: action["x"].as_f64().unwrap(),
                        y: action["y"].as_f64().unwrap(),
                    }),
                    "leave" => driver.pointer_left(),
                    "target" => driver.look_at(match &action["value"] {
                        Value::Null => Target::None,
                        Value::String(value) => match value.as_str() {
                            "pointer" => Target::Pointer,
                            "rest" => Target::Rest,
                            "element" => Target::Element(watched),
                            _ => panic!("unknown target"),
                        },
                        point => Target::Point(serde_json::from_value(point.clone()).unwrap()),
                    }),
                    "measure" => {
                        watched = serde_json::from_value(action["watched"].clone()).unwrap();
                        driver.remeasure(
                            serde_json::from_value(action["bounds"].clone()).unwrap(),
                            travel,
                            Some(watched),
                        );
                    }
                    "replace" => driver.replace_face(Some(face(&action["index"]))),
                    "enabled" => driver.set_enabled(action["value"].as_bool().unwrap()),
                    "stop" => driver.stop(),
                    kind => panic!("unknown action {kind}"),
                }
                assert_eq!(
                    driver.needs_frame(),
                    row["scheduled"].as_bool().unwrap(),
                    "{action}"
                );
                if !row["raw"].is_null() {
                    close(
                        &json!([driver.position.x, driver.position.y, driver.hold]),
                        &row["raw"],
                    );
                }
                close(&json!(driver.frame()), &row["output"]);
            }
        }
        for row in fixture["rounding"].as_array().unwrap() {
            close(
                &json!(fixed(
                    row["value"].as_f64().unwrap(),
                    row["digits"].as_u64().unwrap() as u32
                )),
                &row["result"],
            );
        }
    }
}
