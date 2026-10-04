use std::sync::Arc;

use blobatar_core::{Avatar, Background, Options, traits::Override};
use blobatar_gpui::{
    Blobatar, Drawing,
    gpui::{Context, Render, Window, div, prelude::*, px, rgb},
};

pub struct Matrix {
    size: f32,
    background: String,
    light: bool,
    drawings: Vec<(String, Arc<Drawing>)>,
}

impl Matrix {
    pub fn from_args(args: &[String]) -> Result<Option<Self>, &'static str> {
        if args.is_empty() {
            return Ok(None);
        }
        let usage = "Usage: blobatar-demo [--matrix {24|40|64|128|256} {none|circle|squircle|square} {light|dark}]";
        if args.len() != 4 || args[0] != "--matrix" {
            return Err(usage);
        }
        let size: u16 = args[1].parse().map_err(|_| usage)?;
        if ![24, 40, 64, 128, 256].contains(&size) {
            return Err(usage);
        }
        if !["none", "circle", "squircle", "square"].contains(&args[2].as_str())
            || !["light", "dark"].contains(&args[3].as_str())
        {
            return Err(usage);
        }
        let drawings = [0.1, 0.35, 0.55, 0.65, 0.75, 0.82, 0.89, 0.93, 0.965, 0.99]
            .into_iter()
            .map(|shape| {
                let mut options = Options {
                    background: Some(if args[2] == "none" {
                        Background::Enabled(false)
                    } else {
                        Background::Kind(args[2].clone())
                    }),
                    ..Default::default()
                };
                options
                    .traits
                    .insert("shape".into(), Override::Fixed(shape));
                let avatar = Avatar::new("ひろと", &options);
                (
                    avatar.layout.shape.clone(),
                    Arc::new(Drawing::new(&avatar, &options)),
                )
            })
            .collect();
        Ok(Some(Self {
            size: f32::from(size),
            background: args[2].clone(),
            light: args[3] == "light",
            drawings,
        }))
    }
}

impl Render for Matrix {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(if self.light { 0xf4f5f7 } else { 0x111318 }))
            .text_color(rgb(if self.light { 0x111318 } else { 0xe7eaf0 }))
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(24.0))
            .child(div().text_2xl().child("Blobatar / Native rendering matrix"))
            .child(format!(
                "Seed: ひろと · {}px · {} · {} surface",
                self.size,
                self.background,
                if self.light { "light" } else { "dark" }
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(16.0))
                    .children(self.drawings.iter().map(|(shape, drawing)| {
                        div()
                            .w(px(272.0))
                            .h(px(312.0))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                                div()
                                    .w(px(272.0))
                                    .h(px(272.0))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Blobatar::from_drawing(drawing.clone()).size(self.size)),
                            )
                            .child(div().text_sm().child(shape.clone()))
                    })),
            )
    }
}
