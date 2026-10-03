use std::{sync::Arc, time::Instant};

use blobatar_core::{Avatar, Expression, Options, color::Palette, traits::Traits};
use blobatar_motion::{
    ease,
    idle::{IdleSeeds, idle_at},
    morph::{Fill, Morph},
    transform::{Affine, frame_transforms_with_hover},
};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*};

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
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
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

impl Render for AnimatedBlobatar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = self.now();
        let still = self.reduced_motion || self.mode == Animate::Never;
        let shown = self.morph.sample(now);
        let amplitude = if still { 0.0 } else { self.amplitude(now) };
        let mut idle = idle_at(
            self.seeds,
            now,
            amplitude,
            if still { 0.0 } else { shown.pose.shake },
        );
        if still {
            idle.rockp = 1.0;
        }
        let lift = if still { 0.0 } else { self.lift(now) };
        let hover = Affine::translate(50.0, 50.0 - 1.5 * lift)
            .compose(Affine::scale(1.0 + 0.04 * lift, 1.0 + 0.04 * lift))
            .compose(Affine::translate(-50.0, -50.0));
        let transforms = frame_transforms_with_hover(&self.avatar.layout, shown.pose, idle, hover);
        if !still
            && (amplitude > 0.0
                || self.amplitude_to > 0.0
                || shown.pose.shake != 0.0
                || shown.pose.rock != 0.0
                || self.morph.is_running(now)
                || now < self.lift_start_ms + 220.0)
        {
            window.request_animation_frame();
        }
        let mut blobatar = Blobatar::from_drawing(if still {
            self.still_drawing.clone()
        } else {
            self.drawing.clone()
        })
        .size(self.size);
        if !still {
            blobatar.frame = Some(PaintFrame {
                transforms,
                head: rgb(shown.fill.head),
                eye: rgb(shown.fill.eye),
            });
        }
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
