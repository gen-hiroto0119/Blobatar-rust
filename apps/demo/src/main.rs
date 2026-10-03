use blobatar_core::{Avatar, Background, Expression, Options, traits::Override};
use blobatar_gpui::{
    Blobatar, Drawing,
    gpui::{
        self, App, Application, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div,
        prelude::*, px, rgb, size,
    },
};
use std::sync::Arc;

struct Demo {
    drawings: Vec<Arc<Drawing>>,
    expressions: Vec<(Expression, Arc<Drawing>)>,
}

impl Demo {
    fn new() -> Self {
        let drawings = [0.1, 0.35, 0.55, 0.65, 0.75, 0.82, 0.89, 0.93, 0.965, 0.99]
            .into_iter()
            .map(|shape| {
                let mut options = Options {
                    background: Some(Background::Enabled(true)),
                    ..Default::default()
                };
                options
                    .traits
                    .insert("shape".into(), Override::Fixed(shape));
                Arc::new(Drawing::new(&Avatar::new("ひろと", &options), &options))
            })
            .collect();
        let expressions = Expression::ALL
            .into_iter()
            .map(|expression| {
                let options = Options {
                    expression: Some(expression),
                    background: Some(Background::Kind("squircle".into())),
                    ..Default::default()
                };
                (
                    expression,
                    Arc::new(Drawing::new(&Avatar::new("ひろと", &options), &options)),
                )
            })
            .collect();
        Self {
            drawings,
            expressions,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x111318))
            .text_color(rgb(0xe7eaf0))
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(div().text_2xl().child("Blobatar / Native Rust"))
            .child(
                div()
                    .text_sm()
                    .child("生成2 · 10形状 / Shapes · GPUI vector paths"),
            )
            .child(
                div().flex().flex_wrap().gap_2().children(
                    self.drawings
                        .iter()
                        .map(|drawing| Blobatar::from_drawing(drawing.clone()).size(80.0)),
                ),
            )
            .child(div().text_sm().child("14の静止表情 / Static expressions"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .children(self.expressions.iter().map(|(expression, drawing)| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .rounded_lg()
                            .bg(rgb(0x1b1e26))
                            .child(Blobatar::from_drawing(drawing.clone()).size(100.0))
                            .child(div().text_xs().child(expression.name()))
                    })),
            )
            .child(
                div().text_sm().child(
                    "SVGとネイティブ描画で同じ形・色・表情を共有。動作と操作画面は実装中です。",
                ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(980.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Blobatar".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Demo::new()),
        )
        .expect("open Blobatar window");
        cx.activate(true);
    });
}
