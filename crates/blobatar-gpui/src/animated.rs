use std::{sync::Arc, time::Instant};

use blobatar_core::{
    Avatar, Expression, Options, color::Palette, geometry::Bounds as FaceBounds, traits::Traits,
};
use blobatar_motion::{
    driver::{GazeDriver, Target},
    ease,
    gaze::Mark,
    idle::{IdleSeeds, idle_at},
    morph::{Fill, Morph},
    survey::{self, Face},
    transform::{Affine, frame_transforms_with_gaze},
};
use gpui::{
    Bounds, Context, DispatchPhase, IntoElement, MouseExitEvent, MouseMoveEvent, Pixels, Render,
    Window, div, prelude::*,
};

use crate::{Blobatar, Drawing, PaintFrame, parse_color};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Animate {
    Never,
    Hover,
    Always,
}

/// Retains neutral geometry, palette endpoints, and the interrupted morph.
pub struct AnimatedBlobatar {
    avatar: Avatar,
    drawing: Arc<Drawing>,
    still_drawing: Arc<Drawing>,
    options: Options,
    seeds: IdleSeeds,
    morph: Morph,
    expression: Expression,
    epoch: Instant,
    size: f32,
    mode: Animate,
    reduced_motion: bool,
    hovered: bool,
    amplitude_from: f64,
    amplitude_to: f64,
    amplitude_start_ms: f64,
    lift_from: f64,
    lift_start_ms: f64,
    face: Face,
    viewport: FaceBounds,
    gaze: Option<GazeDriver>,
    gaze_travel: f64,
}

impl AnimatedBlobatar {
    pub fn new(name: &str, options: &Options) -> Self {
        let expression = options.expression.unwrap_or_default();
        let mut neutral = options.clone();
        neutral.expression = None;
        let avatar = Avatar::new(name, &neutral);
        let drawing = Arc::new(Drawing::new(&avatar, &neutral));
        let traits = Traits::new(name, options.normalize, &options.traits);
        let seeds = IdleSeeds::new(&traits);
        let morph = Morph::new(expression, fill(&expression.palette(&avatar.palette)));
        let still_drawing = posed_drawing(&avatar, options, expression);
        let face = survey::survey(&avatar.layout).unwrap_or(Face {
            marks: Vec::new(),
            rx: 50.0,
            ry: 50.0,
        });
        let viewport = FaceBounds {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        let gaze = Some(GazeDriver::new(face.clone(), viewport, 0.0));
        Self {
            avatar,
            drawing,
            still_drawing,
            options: options.clone(),
            seeds,
            morph,
            expression,
            epoch: Instant::now(),
            size: options.size.unwrap_or(64.0) as f32,
            mode: Animate::Hover,
            reduced_motion: false,
            hovered: false,
            amplitude_from: 0.0,
            amplitude_to: 0.0,
            amplitude_start_ms: -400.0,
            lift_from: 0.0,
            lift_start_ms: -220.0,
            face,
            viewport,
            gaze,
            gaze_travel: 0.0,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Full excursion in the 100-unit viewBox; zero by default, like gaze.css.
    pub fn gaze_travel(mut self, travel: f64) -> Self {
        self.gaze_travel = travel;
        if let Some(gaze) = &mut self.gaze {
            gaze.remeasure(self.viewport, travel, None);
        }
        self
    }

    /// Installs a fresh controller if a previous one was stopped.
    pub fn look_at(&mut self, target: Target, cx: &mut Context<Self>) {
        let gaze = self.gaze.get_or_insert_with(|| {
            GazeDriver::new(self.face.clone(), self.viewport, self.gaze_travel)
        });
        gaze.look_at(target);
        cx.notify();
    }

    /// Hosts supply changed element bounds, or zero bounds when detached.
    pub fn remeasure_gaze_target(&mut self, bounds: FaceBounds, cx: &mut Context<Self>) {
        if let Some(gaze) = &mut self.gaze {
            gaze.remeasure(self.viewport, self.gaze_travel, Some(bounds));
            cx.notify();
        }
    }

    pub fn stop_gaze(&mut self, cx: &mut Context<Self>) {
        self.gaze = None;
        cx.notify();
    }

    pub fn set_expression(&mut self, expression: Expression, cx: &mut Context<Self>) {
        if self.expression == expression {
            return;
        }
        self.expression = expression;
        self.still_drawing = posed_drawing(&self.avatar, &self.options, expression);
        self.morph.set_expression(
            expression,
            fill(&expression.palette(&self.avatar.palette)),
            self.now(),
        );
        if self.reduced_motion || self.mode == Animate::Never {
            self.morph.finish();
        }
        cx.notify();
    }

    pub fn set_animate(&mut self, mode: Animate, cx: &mut Context<Self>) {
        let now = self.now();
        self.amplitude_from = self.amplitude(now);
        self.mode = mode;
        self.amplitude_to = if mode == Animate::Always || (mode == Animate::Hover && self.hovered) {
            1.0
        } else {
            0.0
        };
        self.amplitude_start_ms = now;
        if mode == Animate::Never {
            self.morph.finish();
        }
        cx.notify();
    }

    pub fn set_reduced_motion(&mut self, reduced: bool, cx: &mut Context<Self>) {
        self.reduced_motion = reduced;
        if reduced {
            self.morph.finish();
        }
        cx.notify();
    }

    fn now(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64() * 1000.0
    }

    fn amplitude(&self, now: f64) -> f64 {
        let progress = ease::ease_out(((now - self.amplitude_start_ms) / 400.0).clamp(0.0, 1.0));
        self.amplitude_from + (self.amplitude_to - self.amplitude_from) * progress
    }

    fn lift(&self, now: f64) -> f64 {
        let duration = if self.hovered { 220.0 } else { 160.0 };
        let progress = ease::bezier(
            ((now - self.lift_start_ms) / duration).clamp(0.0, 1.0),
            [0.23, 1.0, 0.32, 1.0],
        );
        let target = if self.hovered { 1.0 } else { 0.0 };
        self.lift_from + (target - self.lift_from) * progress
    }

    fn hover(&mut self, hovered: bool, cx: &mut Context<Self>) {
        let now = self.now();
        self.lift_from = self.lift(now);
        self.lift_start_ms = now;
        self.amplitude_from = self.amplitude(now);
        self.amplitude_start_ms = now;
        self.hovered = hovered;
        self.amplitude_to =
            if self.mode == Animate::Always || (self.mode == Animate::Hover && hovered) {
                1.0
            } else {
                0.0
            };
        cx.notify();
    }
}

impl AnimatedBlobatar {
    fn paint_frame(&mut self, bounds: Bounds<Pixels>, window: &mut Window) -> Option<PaintFrame> {
        let now = self.now();
        let still = self.reduced_motion || self.mode == Animate::Never;
        let viewport = FaceBounds {
            x: f64::from(f32::from(bounds.origin.x)),
            y: f64::from(f32::from(bounds.origin.y)),
            width: f64::from(f32::from(bounds.size.width)),
            height: f64::from(f32::from(bounds.size.height)),
        };
        if let Some(gaze) = &mut self.gaze {
            if self.viewport != viewport {
                gaze.remeasure(viewport, self.gaze_travel, None);
            }
            gaze.set_enabled(!still);
            gaze.tick(now);
        }
        self.viewport = viewport;
        if still {
            return None;
        }
        let gaze = self.gaze.as_ref().map(GazeDriver::frame);
        let shown = self.morph.sample(now);
        let amplitude = self.amplitude(now);
        let idle = idle_at(
            self.seeds
                .with_gaze_hold(gaze.map_or(0.0, |frame| frame.hold)),
            now,
            amplitude,
            shown.pose.shake,
        );
        let lift = self.lift(now);
        let hover = Affine::translate(50.0, 50.0 - 1.5 * lift)
            .compose(Affine::scale(1.0 + 0.04 * lift, 1.0 + 0.04 * lift))
            .compose(Affine::translate(-50.0, -50.0));
        let transforms = frame_transforms_with_gaze(
            &self.avatar.layout,
            shown.pose,
            idle,
            hover,
            gaze.map_or(&[], |frame| frame.eyes.as_slice()),
        );
        if amplitude > 0.0
            || self.amplitude_to > 0.0
            || shown.pose.shake != 0.0
            || shown.pose.rock != 0.0
            || self.morph.is_running(now)
            || now < self.lift_start_ms + 220.0
            || self.gaze.as_ref().is_some_and(GazeDriver::needs_frame)
        {
            window.request_animation_frame();
        }
        Some(PaintFrame {
            transforms,
            head: rgb(shown.fill.head),
            eye: rgb(shown.fill.eye),
        })
    }
}

impl Render for AnimatedBlobatar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let still = self.reduced_motion || self.mode == Animate::Never;
        let mut blobatar = Blobatar::from_drawing(if still {
            self.still_drawing.clone()
        } else {
            self.drawing.clone()
        })
        .size(self.size);
        let entity = cx.entity().downgrade();
        blobatar.prepare = Some(Box::new(move |bounds, window, cx| {
            entity
                .update(cx, |this, _| this.paint_frame(bounds, window))
                .ok()
                .flatten()
        }));
        let entity = cx.entity().downgrade();
        blobatar.listen = Some(Box::new(move |window, cx| {
            let Some(view) = entity.upgrade() else {
                return;
            };
            if !view
                .read(cx)
                .gaze
                .as_ref()
                .is_some_and(GazeDriver::is_enabled)
            {
                return;
            }
            let moved = entity.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    let _ = moved.update(cx, |this, cx| {
                        if let Some(gaze) = &mut this.gaze {
                            gaze.pointer_moved(Mark {
                                x: f64::from(f32::from(event.position.x)),
                                y: f64::from(f32::from(event.position.y)),
                            });
                            cx.notify();
                        }
                    });
                }
            });
            window.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    let _ = entity.update(cx, |this, cx| {
                        if let Some(gaze) = &mut this.gaze {
                            gaze.pointer_left();
                            cx.notify();
                        }
                    });
                }
            });
        }));
        div()
            .id("animated-blobatar")
            .child(blobatar)
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| this.hover(*hovered, cx)))
    }
}

fn posed_drawing(avatar: &Avatar, options: &Options, expression: Expression) -> Arc<Drawing> {
    let pose = expression.pose();
    let posed = Avatar {
        layout: blobatar_core::pose::bake(&avatar.layout, pose),
        palette: expression.palette(&avatar.palette),
        body_offset_y: pose.body_offset_y,
    };
    Arc::new(Drawing::new(&posed, options))
}

fn fill(palette: &Palette) -> Fill {
    let bytes = |value: Option<&str>| {
        let color = parse_color(value);
        [color.r, color.g, color.b].map(|v| (v * 255.0).round() as u8)
    };
    Fill {
        head: bytes(palette.head.as_deref()),
        eye: bytes(palette.eye.as_deref()),
    }
}

fn rgb([r, g, b]: [u8; 3]) -> gpui::Rgba {
    gpui::rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
}
