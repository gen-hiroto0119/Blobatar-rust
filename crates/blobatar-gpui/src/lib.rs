use std::sync::Arc;

use blobatar_core::{Avatar, Command, Options, Path};
use gpui::{
    App, Bounds, IntoElement, PathBuilder, Pixels, RenderOnce, Rgba, Styled, Window, canvas, point,
    px, quad, rgb, size,
};

pub use gpui;

/// Share a drawing between rerenders so name-derived geometry is not regenerated.
pub struct Drawing {
    paths: Vec<(Path, Rgba)>,
    circles: Vec<(f64, f64, f64, Rgba)>,
    backdrop: Option<(Path, Rgba)>,
    body_offset_y: f64,
}

impl Drawing {
    pub fn new(avatar: &Avatar, options: &Options) -> Self {
        let head = parse_color(avatar.palette.head.as_deref());
        let eye = parse_color(avatar.palette.eye.as_deref());
        let mut paths = avatar
            .layout
            .extra_paths()
            .iter()
            .cloned()
            .map(|path| (path, head))
            .collect::<Vec<_>>();
        paths.push((avatar.layout.body_path(), head));
        paths.extend(
            avatar
                .layout
                .eye_paths()
                .into_iter()
                .map(|path| (path, eye)),
        );
        let circles = avatar
            .layout
            .petals
            .iter()
            .map(|petal| (petal.cx, petal.cy, petal.r, head))
            .collect();
        let backdrop = blobatar_core::avatar::backdrop_path(options.background.as_ref())
            .map(|path| (path, parse_color(avatar.palette.bg.as_deref())));
        Self {
            paths,
            circles,
            backdrop,
            body_offset_y: avatar.body_offset_y,
        }
    }
}

fn parse_color(hex: Option<&str>) -> Rgba {
    Rgba::try_from(hex.unwrap_or("#000000")).unwrap_or_else(|_| rgb(0))
}

#[derive(IntoElement)]
pub struct Blobatar {
    drawing: Arc<Drawing>,
    size: Pixels,
}

impl Blobatar {
    pub fn new(name: &str) -> Self {
        Self::with_options(name, &Options::default())
    }

    pub fn with_options(name: &str, options: &Options) -> Self {
        let avatar = Avatar::new(name, options);
        Self::from_drawing(Arc::new(Drawing::new(&avatar, options)))
    }

    pub fn from_drawing(drawing: Arc<Drawing>) -> Self {
        Self {
            drawing,
            size: px(64.0),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = px(size);
        self
    }
}

impl RenderOnce for Blobatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let drawing = self.drawing;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let scale = f64::from(f32::from(bounds.size.width)) / 100.0;
                if let Some((path, color)) = &drawing.backdrop {
                    paint_path(window, bounds, scale, path, *color);
                }
                let bounds = Bounds {
                    origin: bounds.origin
                        + point(px(0.0), px((drawing.body_offset_y * scale) as f32)),
                    size: bounds.size,
                };
                for &(cx, cy, radius, color) in &drawing.circles {
                    // A circular native quad preserves circles rather than substituting superellipses.
                    let radius = px((radius * scale) as f32);
                    window.paint_quad(quad(
                        Bounds {
                            origin: bounds.origin
                                + point(
                                    px((cx * scale) as f32) - radius,
                                    px((cy * scale) as f32) - radius,
                                ),
                            size: size(radius * 2.0, radius * 2.0),
                        },
                        radius,
                        color,
                        px(0.0),
                        gpui::transparent_black(),
                        Default::default(),
                    ));
                }
                for (path, color) in &drawing.paths {
                    paint_path(window, bounds, scale, path, *color);
                }
            },
        )
        .size(self.size)
    }
}

fn paint_path(window: &mut Window, bounds: Bounds<Pixels>, scale: f64, path: &Path, color: Rgba) {
    let position =
        |x: f64, y: f64| bounds.origin + point(px((x * scale) as f32), px((y * scale) as f32));
    let mut builder = PathBuilder::fill();
    let mut current = (0.0, 0.0);
    let mut start = current;
    for command in &path.commands {
        match *command {
            Command::MoveTo { x, y } => {
                builder.move_to(position(x, y));
                current = (x, y);
                start = current;
            }
            Command::LineTo { x, y } => {
                builder.line_to(position(x, y));
                current = (x, y);
            }
            Command::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                builder.cubic_bezier_to(position(x, y), position(x1, y1), position(x2, y2));
                current = (x, y);
            }
            Command::QuadraticTo { x1, y1, x, y } => {
                builder.curve_to(position(x, y), position(x1, y1));
                current = (x, y);
            }
            Command::HorizontalTo { x } => {
                builder.line_to(position(x, current.1));
                current.0 = x;
            }
            Command::VerticalTo { y } => {
                builder.line_to(position(current.0, y));
                current.1 = y;
            }
            Command::Close => {
                builder.close();
                current = start;
            }
        }
    }
    match builder.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => eprintln!("Unable to tessellate avatar path: {error}"),
    }
}
