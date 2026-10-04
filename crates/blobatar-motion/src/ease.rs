pub fn bezier(x: f64, [x1, y1, x2, y2]: [f64; 4]) -> f64 {
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;
    let mut t = x;
    for _ in 0..8 {
        let error = ((ax * t + bx) * t + cx) * t - x;
        if error.abs() < 1e-5 {
            break;
        }
        let derivative = (3.0 * ax * t + 2.0 * bx) * t + cx;
        if derivative.abs() < 1e-6 {
            break;
        }
        t -= error / derivative;
    }
    ((ay * t + by) * t + cy) * t
}

pub fn in_out(x: f64) -> f64 {
    bezier(x, [0.42, 0.0, 0.58, 1.0])
}
pub fn ease_in(x: f64) -> f64 {
    bezier(x, [0.42, 0.0, 1.0, 1.0])
}
pub fn ease_out(x: f64) -> f64 {
    bezier(x, [0.0, 0.0, 0.58, 1.0])
}
pub fn morph_in(x: f64) -> f64 {
    bezier(x, [0.45, 0.05, 0.5, 1.0])
}
