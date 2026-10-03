use crate::gaze::Mark;
use blobatar_core::{
    Layout,
    geometry::{Bounds, Outline},
    shape::rounded,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Face {
    pub marks: Vec<Mark>,
    pub rx: f64,
    pub ry: f64,
}

/// Fit the gaze surface once per neutral layout, independently of its viewport.
pub fn survey(layout: &Layout) -> Option<Face> {
    let mut outlines: Vec<_> = layout.extra_paths().iter().map(Outline::new).collect();
    outlines.push(Outline::new(&layout.body_path()));
    let circles: Vec<_> = layout
        .petals
        .iter()
        .map(|p| (rounded(p.cx), rounded(p.cy), rounded(p.r)))
        .collect();
    let bounds = outlines
        .iter()
        .filter_map(|p| p.bounds)
        .chain(circles.iter().map(|&(x, y, r)| Bounds {
            x: x - r,
            y: y - r,
            width: 2.0 * r,
            height: 2.0 * r,
        }))
        .reduce(Bounds::union)?;
    let hit = |x: f64, y: f64| {
        outlines.iter().any(|p| p.contains(x, y))
            || circles
                .iter()
                .any(|&(px, py, r)| (x - px).hypot(y - py) <= r + 1e-10)
    };
    let eyes: Vec<_> = layout
        .eye_paths()
        .iter()
        .filter_map(|p| Outline::new(p).bounds)
        .collect();
    fit_face(bounds, &eyes, hit)
}

/// The upstream f64 fitter, separated from the renderer's bounds/fill queries.
pub fn fit_face(
    bounds: Bounds,
    eyes: &[Bounds],
    mut contains: impl FnMut(f64, f64) -> bool,
) -> Option<Face> {
    if eyes.is_empty() {
        return None;
    }
    let [cx, cy] = bounds.center();
    let mut hit = |t: f64, c: f64, s: f64| {
        contains(
            cx + t * bounds.width / 2.0 * c,
            cy + t * bounds.height / 2.0 * s,
        )
    };
    let mut fit = 1.0_f64;
    for ray in 0..16 {
        let angle = f64::from(ray) / 16.0 * std::f64::consts::PI * 2.0;
        let (s, c) = angle.sin_cos();
        if hit(1.0, c, s) {
            continue;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if hit(mid, c, s) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        fit = fit.min(lo);
    }
    let ix = eyes.iter().map(|b| b.width / 2.0).fold(0.0, f64::max);
    let iy = eyes.iter().map(|b| b.height / 2.0).fold(0.0, f64::max);
    let mut rx = (bounds.width / 2.0 * fit - ix).max(1.0);
    let mut ry = (bounds.height / 2.0 * fit - iy).max(1.0);
    let centers: Vec<_> = eyes
        .iter()
        .map(|b| {
            let [x, y] = b.center();
            [x - cx, y - cy]
        })
        .collect();
    let need = centers
        .iter()
        .map(|[x, y]| (x / rx).hypot(y / ry))
        .fold(0.0, f64::max);
    if need > 0.85 {
        rx *= need / 0.85;
        ry *= need / 0.85;
    }
    Some(Face {
        marks: centers
            .into_iter()
            .map(|[x, y]| Mark {
                x: x / rx,
                y: y / ry,
            })
            .collect(),
        rx,
        ry,
    })
}
