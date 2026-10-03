use blobatar_core::{Avatar, Background, Expression, Options, traits::Override};
use blobatar_gpui::{
    Animate, AnimatedBlobatar, Blobatar, Drawing,
    gpui::{
        self, App, Application, Bounds, Context, Entity, Render, Window, WindowBounds,
        WindowOptions, div, prelude::*, px, rgb, size,
    },
};
use std::sync::Arc;
mod matrix;

struct Demo {
    drawings: Vec<Arc<Drawing>>,
    expressions: Vec<(Expression, Arc<Drawing>)>,
    preview: Entity<AnimatedBlobatar>,
    selected: Expression,
    mode: Animate,
    reduced_motion: bool,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
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
        let preview = cx.new(|_| {
            AnimatedBlobatar::new(
                "ひろと",
                &Options {
                    background: Some(Background::Kind("squircle".into())),
                    ..Default::default()
                },
            )
            .size(140.0)
        });
        Self {
            drawings,
            expressions,
            preview,
            selected: Expression::Idle,
            mode: Animate::Hover,
            reduced_motion: false,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("demo")
            .size_full()
            .overflow_y_scroll()
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
                div()
                    .text_sm()
                    .child("表情の切り替え / Morph · hover to wake up"),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .items_center()
                    .child(self.preview.clone())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .flex_1()
                            .child(div().flex().flex_wrap().gap_2().children(
                                Expression::ALL.into_iter().map(|expression| {
                                    div()
                                        .id(expression.name())
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .bg(rgb(if self.selected == expression {
                                            0x354a70
                                        } else {
                                            0x1b1e26
                                        }))
                                        .text_sm()
                                        .child(expression.name())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected = expression;
                                            this.preview.update(cx, |preview, cx| {
                                                preview.set_expression(expression, cx)
                                            });
                                            cx.notify();
                                        }))
                                }),
                            ))
                            .child(
                                div().flex().flex_wrap().gap_2().children(
                                    [
                                        (Animate::Never, "停止 / Off"),
                                        (Animate::Hover, "ホバー / Hover"),
                                        (Animate::Always, "常時 / Always"),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(
                                        |(index, (mode, label))| {
                                            div()
                                                .id(("mode", index))
                                                .px_3()
                                                .py_2()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .bg(rgb(if self.mode == mode {
                                                    0x354a70
                                                } else {
                                                    0x1b1e26
                                                }))
                                                .text_sm()
                                                .child(label)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.mode = mode;
                                                    this.preview.update(cx, |preview, cx| {
                                                        preview.set_animate(mode, cx)
                                                    });
                                                    cx.notify();
                                                }))
                                        },
                                    ),
                                ),
                            )
                            .child(
                                div()
                                    .id("reduced-motion")
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .bg(rgb(if self.reduced_motion {
                                        0x354a70
                                    } else {
                                        0x1b1e26
                                    }))
                                    .text_sm()
                                    .child(if self.reduced_motion {
                                        "動きを減らす: ON / Reduced motion"
                                    } else {
                                        "動きを減らす: OFF / Reduced motion"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.reduced_motion = !this.reduced_motion;
                                        this.preview.update(cx, |preview, cx| {
                                            preview.set_reduced_motion(this.reduced_motion, cx)
                                        });
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .child("ネイティブ動作の確認用。視線入力・エディター・保存/APIは未実装です。"),
            )
    }
}

fn main() {
    let matrix = match matrix::Matrix::from_args(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(matrix) => matrix,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    Application::new().run(move |cx: &mut App| {
        let dimensions = if matrix.is_some() {
            size(px(1500.0), px(850.0))
        } else {
            size(px(1100.0), px(940.0))
        };
        let bounds = Bounds::centered(None, dimensions, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Blobatar".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Some(matrix) = matrix {
            cx.open_window(options, |_, cx| cx.new(|_| matrix))
                .expect("open rendering matrix");
        } else {
            cx.open_window(options, |_, cx| cx.new(Demo::new))
                .expect("open Blobatar window");
        }
        cx.activate(true);
    });
}
