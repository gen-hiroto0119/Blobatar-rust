use crate::{Command, Path, shape::rounded};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            width: (self.x + self.width).max(other.x + other.width) - x,
            height: (self.y + self.height).max(other.y + other.height) - y,
        }
    }

    pub fn center(self) -> [f64; 2] {
        [self.x + self.width / 2.0, self.y + self.height / 2.0]
    }
}

#[derive(Clone, Debug)]
struct Curve {
    points: [[f64; 2]; 4],
    y_stops: Vec<f64>,
}

impl Curve {
    fn new(points: [[f64; 2]; 4]) -> Self {
        let mut curve = Self {
            points,
            y_stops: Vec::new(),
        };
        curve.y_stops = curve.stops(1);
        curve
    }

    fn at(&self, t: f64, axis: usize) -> f64 {
        let [a, b, c, d] = self.points.map(|p| p[axis]);
        let s = 1.0 - t;
        s * s * s * a + 3.0 * s * s * t * b + 3.0 * s * t * t * c + t * t * t * d
    }

    fn stops(&self, axis: usize) -> Vec<f64> {
        let [p0, p1, p2, p3] = self.points.map(|p| p[axis]);
        let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
        let b = 2.0 * (p0 - 2.0 * p1 + p2);
        let c = p1 - p0;
        let mut stops = vec![0.0, 1.0];
        let mut add = |t| {
            if t > 0.0 && t < 1.0 {
                stops.push(t);
            }
        };
        if a == 0.0 {
            if b != 0.0 {
                add(-c / b);
            }
        } else {
            let discriminant = b * b - 4.0 * a * c;
            if discriminant >= 0.0 {
                // The other quadratic root avoids cancellation when this is nearly linear.
                let q = -0.5 * (b + discriminant.sqrt().copysign(b));
                add(q / a);
                if q != 0.0 {
                    add(c / q);
                }
            }
        }
        stops.sort_by(f64::total_cmp);
        stops.dedup();
        stops
    }

    fn bounds(&self) -> Bounds {
        let extent = |axis| {
            self.stops(axis)
                .into_iter()
                .map(|t| self.at(t, axis))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                })
        };
        let (x, right) = extent(0);
        let (y, bottom) = extent(1);
        Bounds {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }
    }

    fn winding(&self, x: f64, y: f64) -> (i32, bool) {
        let mut winding = 0;
        for interval in self.y_stops.windows(2) {
            let mut lo = interval[0];
            let mut hi = interval[1];
            let start = self.at(lo, 1);
            let end = self.at(hi, 1);
            let upward = end > start;
            if start == end && y == start {
                let bounds = self.bounds();
                if x >= bounds.x && x <= bounds.x + bounds.width {
                    return (0, true);
                }
            }
            if y < start.min(end) || y > start.max(end) || start == end {
                continue;
            }
            // Half-open intervals count shared vertices only once.
            for _ in 0..48 {
                let mid = (lo + hi) / 2.0;
                if (self.at(mid, 1) < y) == upward {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let crossing = self.at((lo + hi) / 2.0, 0);
            if (crossing - x).abs() < 1e-10 {
                return (0, true);
            }
            if y < start.max(end) && crossing > x {
                winding += if upward { 1 } else { -1 };
            }
        }
        (winding, false)
    }
}

/// Bounds and nonzero-fill queries on the same rounded coordinates as the SVG.
#[derive(Clone, Debug)]
pub struct Outline {
    curves: Vec<Curve>,
    pub bounds: Option<Bounds>,
}

impl Outline {
    pub fn new(path: &Path) -> Self {
        let mut curves = Vec::new();
        let mut current = [0.0, 0.0];
        let mut start = None;
        let point = |x, y| [rounded(x), rounded(y)];
        let line = |curves: &mut Vec<Curve>, a, b| {
            if a != b {
                curves.push(Curve::new([a, a, b, b]));
            }
        };
        for command in &path.commands {
            match *command {
                Command::MoveTo { x, y } => {
                    if let Some(start) = start {
                        line(&mut curves, current, start);
                    }
                    current = point(x, y);
                    start = Some(current);
                }
                Command::LineTo { x, y } => {
                    let end = point(x, y);
                    line(&mut curves, current, end);
                    current = end;
                }
                Command::HorizontalTo { x } => {
                    let end = [rounded(x), current[1]];
                    line(&mut curves, current, end);
                    current = end;
                }
                Command::VerticalTo { y } => {
                    let end = [current[0], rounded(y)];
                    line(&mut curves, current, end);
                    current = end;
                }
                Command::CubicTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let end = point(x, y);
                    curves.push(Curve::new([current, point(x1, y1), point(x2, y2), end]));
                    current = end;
                }
                Command::QuadraticTo { x1, y1, x, y } => {
                    let control = point(x1, y1);
                    let end = point(x, y);
                    let first =
                        std::array::from_fn(|i| current[i] + (control[i] - current[i]) * 2.0 / 3.0);
                    let second =
                        std::array::from_fn(|i| end[i] + (control[i] - end[i]) * 2.0 / 3.0);
                    curves.push(Curve::new([current, first, second, end]));
                    current = end;
                }
                Command::Close => {
                    if let Some(start) = start {
                        line(&mut curves, current, start);
                        current = start;
                    }
                }
            }
        }
        if let Some(start) = start {
            line(&mut curves, current, start);
        }
        let bounds = curves.iter().map(Curve::bounds).reduce(Bounds::union);
        Self { curves, bounds }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let mut winding = 0;
        for curve in &self.curves {
            let (crossings, boundary) = curve.winding(x, y);
            if boundary {
                return true;
            }
            winding += crossings;
        }
        winding != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bounds_use_extrema_not_control_points() {
        let curve = Curve::new([[0.0, 0.0], [0.0, 4.0], [4.0, 4.0], [4.0, 0.0]]);
        assert_eq!(
            curve.bounds(),
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 3.0
            }
        );
    }

    #[test]
    fn nearly_quadratic_cubic_keeps_its_small_root() {
        let outline = Outline::new(&Path::new(vec![
            Command::MoveTo { x: 31.36, y: 72.52 },
            Command::QuadraticTo {
                x1: 15.52,
                y1: 73.67,
                x: 22.37,
                y: 60.4,
            },
            Command::Close,
        ]));
        let t = (31.36 - 15.52) / (31.36 - 2.0 * 15.52 + 22.37);
        let expected = (1.0 - t) * (1.0 - t) * 31.36 + 2.0 * (1.0 - t) * t * 15.52 + t * t * 22.37;
        assert!((outline.bounds.unwrap().x - expected).abs() < 1e-12);
    }

    #[test]
    fn nonzero_fill_closes_paths_and_preserves_holes() {
        let outline = Outline::new(&Path::new(vec![
            Command::MoveTo { x: 0.0, y: 0.0 },
            Command::HorizontalTo { x: 10.0 },
            Command::VerticalTo { y: 10.0 },
            Command::HorizontalTo { x: 0.0 },
            Command::MoveTo { x: 2.0, y: 2.0 },
            Command::VerticalTo { y: 8.0 },
            Command::HorizontalTo { x: 8.0 },
            Command::VerticalTo { y: 2.0 },
            Command::Close,
        ]));
        assert!(outline.contains(1.0, 5.0));
        assert!(!outline.contains(5.0, 5.0));
        assert!(!outline.contains(11.0, 5.0));
        assert!(outline.contains(10.0, 5.0));
        assert!(outline.contains(5.0, 10.0));
    }
}
