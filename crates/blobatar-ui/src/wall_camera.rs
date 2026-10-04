#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn scale(self) -> f64 {
        80.0 * self.zoom
    }

    pub fn to_screen(self, view: (f64, f64), cell: (f64, f64)) -> (f64, f64) {
        (
            view.0 / 2.0 + (cell.0 - self.x) * self.scale(),
            view.1 / 2.0 + (cell.1 - self.y) * self.scale(),
        )
    }

    pub fn to_cell(self, view: (f64, f64), screen: (f64, f64)) -> (f64, f64) {
        (
            self.x + (screen.0 - view.0 / 2.0) / self.scale(),
            self.y + (screen.1 - view.1 / 2.0) / self.scale(),
        )
    }

    pub fn cell_under(self, view: (f64, f64), screen: (f64, f64)) -> (i32, i32) {
        let (x, y) = self.to_cell(view, screen);
        // Cells are centred; JavaScript rounds a negative half toward positive infinity.
        ((x + 0.5).floor() as i32, (y + 0.5).floor() as i32)
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.x = (self.x - dx / self.scale()).clamp(-1_000_000.0, 1_000_000.0);
        self.y = (self.y - dy / self.scale()).clamp(-1_000_000.0, 1_000_000.0);
    }

    pub fn zoom_at(&mut self, view: (f64, f64), screen: (f64, f64), factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let before = self.to_cell(view, screen);
        self.zoom = (self.zoom * factor).clamp(0.1, 2.0);
        let after = self.to_cell(view, screen);
        self.x += before.0 - after.0;
        self.y += before.1 - after.1;
    }

    pub fn visible_box(self, view: (f64, f64), margin: i32) -> (i32, i32, i32, i32) {
        let from = self.to_cell(view, (0.0, 0.0));
        let to = self.to_cell(view, view);
        (
            from.0.floor() as i32 - margin,
            from.1.floor() as i32 - margin,
            to.0.ceil() as i32 + margin,
            to.1.ceil() as i32 + margin,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_matches_the_pinned_upstream_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../blobatar-wall/tests/fixtures/reference.json"
        ))
        .unwrap();
        for case in fixture["views"].as_array().unwrap() {
            let input = &case["camera"];
            let mut camera = Camera {
                x: input["x"].as_f64().unwrap(),
                y: input["y"].as_f64().unwrap(),
                zoom: input["zoom"].as_f64().unwrap(),
            };
            let view = (
                case["view"]["width"].as_f64().unwrap(),
                case["view"]["height"].as_f64().unwrap(),
            );
            let screen = (case["sx"].as_f64().unwrap(), case["sy"].as_f64().unwrap());
            assert_eq!(
                camera.cell_under(view, screen),
                (
                    case["cell"]["x"].as_i64().unwrap() as i32,
                    case["cell"]["y"].as_i64().unwrap() as i32
                )
            );
            let bounds = &case["box"];
            assert_eq!(
                camera.visible_box(view, 4),
                (
                    bounds["x0"].as_i64().unwrap() as i32,
                    bounds["y0"].as_i64().unwrap() as i32,
                    bounds["x1"].as_i64().unwrap() as i32,
                    bounds["y1"].as_i64().unwrap() as i32
                )
            );
            camera.zoom_at(view, screen, 1.4);
            for (actual, key) in [(camera.x, "x"), (camera.y, "y"), (camera.zoom, "zoom")] {
                assert!(
                    (actual - case["zoomed"][key].as_f64().unwrap()).abs() < 1e-12,
                    "{case}: {key}"
                );
            }
        }
    }

    #[test]
    fn negative_half_cells_and_panning_match_the_centred_grid() {
        let mut camera = Camera::default();
        let view = (800.0, 600.0);
        assert_eq!(camera.cell_under(view, (360.0, 260.0)), (0, 0));
        assert_eq!(camera.cell_under(view, (359.0, 259.0)), (-1, -1));
        camera.pan(80.0, -160.0);
        assert_eq!(camera.cell_under(view, (400.0, 300.0)), (-1, 2));
        assert_eq!(camera.to_screen(view, (-1.0, 2.0)), (400.0, 300.0));
    }

    #[test]
    fn zoom_keeps_the_pointer_anchor_and_clamps_the_range() {
        let mut camera = Camera::default();
        let view = (800.0, 600.0);
        let pointer = (100.0, 125.0);
        let before = camera.to_cell(view, pointer);
        for factor in [1.4, 100.0, 0.001] {
            camera.zoom_at(view, pointer, factor);
            let after = camera.to_cell(view, pointer);
            assert!((after.0 - before.0).abs() < 1e-12);
            assert!((after.1 - before.1).abs() < 1e-12);
        }
        assert_eq!(camera.zoom, 0.1);
        assert_eq!(Camera::default().visible_box(view, 4), (-9, -8, 9, 8));
    }
}
