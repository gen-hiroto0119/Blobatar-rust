use blobatar_core::{Avatar, Background, Expression, Options, traits::Override};
use blobatar_gpui::{
    Animate, AnimatedBlobatar, Blobatar, Drawing, GazeBounds, GazePoint, GazeTarget, PlaybackRate,
    gpui::{
        self, App, Application, Bounds, Context, Entity, Render, Window, WindowBounds,
        WindowOptions, canvas, div, prelude::*, px, rgb, size,
    },
};
use std::{sync::Arc, time::Duration};
mod matrix;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GazeMode {
    None,
    Pointer,
    Point,
    Element,
    Rest,
    Stop,
}

struct Demo {
    drawings: Vec<Arc<Drawing>>,
    expressions: Vec<(Expression, Arc<Drawing>)>,
    preview: Entity<AnimatedBlobatar>,
    selected: Expression,
    mode: Animate,
    reduced_motion: bool,
    gaze_mode: GazeMode,
    target_bounds: GazeBounds,
    show_target: bool,
    paused: bool,
    playback_rate: PlaybackRate,
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
            .gaze_travel(2.5)
        });
        Self {
            drawings,
            expressions,
            preview,
            selected: Expression::Idle,
            mode: Animate::Hover,
            reduced_motion: false,
            gaze_mode: GazeMode::None,
            target_bounds: GazeBounds {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            show_target: true,
            paused: false,
            playback_rate: PlaybackRate::Normal,
        }
    }

    fn gaze_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(
                [
                    (GazeMode::None, "解除 / None"),
                    (GazeMode::Pointer, "ポインタ / Pointer"),
                    (GazeMode::Point, "クリック位置 / Point"),
                    (GazeMode::Element, "対象部品 / Element"),
                    (GazeMode::Rest, "中央 / Rest"),
                    (GazeMode::Stop, "視線終了 / Stop"),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (mode, label))| {
                    div()
                        .id(("gaze", index))
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(rgb(if self.gaze_mode == mode {
                            0x354a70
                        } else {
                            0x1b1e26
                        }))
                        .text_sm()
                        .child(label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.gaze_mode = mode;
                            let mouse = window.mouse_position();
                            let target = match mode {
                                GazeMode::None | GazeMode::Stop => GazeTarget::None,
                                GazeMode::Pointer => GazeTarget::Pointer,
                                GazeMode::Point => GazeTarget::Point(GazePoint {
                                    x: f64::from(f32::from(mouse.x)),
                                    y: f64::from(f32::from(mouse.y)),
                                }),
                                GazeMode::Element => GazeTarget::Element(this.target_bounds),
                                GazeMode::Rest => GazeTarget::Rest,
                            };
                            this.preview.update(cx, |preview, cx| {
                                if mode == GazeMode::Stop {
                                    preview.stop_gaze(cx);
                                } else {
                                    preview.look_at(target, cx);
                                }
                            });
                            cx.notify();
                        }))
                }),
            )
            .child(
                div()
                    .id("hide-target")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(0x1b1e26))
                    .text_sm()
                    .child(if self.show_target {
                        "対象を隠す / Hide target"
                    } else {
                        "対象を表示 / Show target"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_target = !this.show_target;
                        if !this.show_target {
                            this.target_bounds = GazeBounds {
                                x: 0.0,
                                y: 0.0,
                                width: 0.0,
                                height: 0.0,
                            };
                            if this.gaze_mode == GazeMode::Element {
                                this.preview.update(cx, |preview, cx| {
                                    preview.remeasure_gaze_target(this.target_bounds, cx)
                                });
                            }
                        }
                        cx.notify();
                    })),
            )
    }

    fn playback_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                div()
                    .id("pause")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(0x1b1e26))
                    .text_sm()
                    .child(if self.paused {
                        "再生 / Play"
                    } else {
                        "一時停止 / Pause"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.paused = !this.paused;
                        this.preview
                            .update(cx, |preview, cx| preview.set_paused(this.paused, cx));
                        cx.notify();
                    })),
            )
            .children(
                [
                    (PlaybackRate::Quarter, "0.25×"),
                    (PlaybackRate::Half, "0.5×"),
                    (PlaybackRate::Normal, "1×"),
                    (PlaybackRate::Double, "2×"),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (rate, label))| {
                    div()
                        .id(("playback-rate", i))
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .bg(rgb(if self.playback_rate == rate {
                            0x354a70
                        } else {
                            0x1b1e26
                        }))
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.playback_rate = rate;
                            this.preview
                                .update(cx, |preview, cx| preview.set_playback_rate(rate, cx));
                            cx.notify();
                        }))
                }),
            )
            .children(
                [0_u64, 1234, 3000]
                    .into_iter()
                    .enumerate()
                    .map(|(i, time)| {
                        div()
                            .id(("idle-time", i))
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(rgb(0x1b1e26))
                            .text_sm()
                            .child(format!("Idle t={time}ms"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.paused = true;
                                this.preview.update(cx, |preview, cx| {
                                    preview.seek_idle(Duration::from_millis(time), cx)
                                });
                                cx.notify();
                            }))
                    }),
            )
    }

    fn gaze_marker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        div()
            .relative()
            .w_24()
            .h_12()
            .rounded_md()
            .bg(rgb(0x665021))
            .text_sm()
            .p_2()
            .child("視線の対象 / Target")
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let measured = GazeBounds {
                            x: f64::from(f32::from(bounds.origin.x)),
                            y: f64::from(f32::from(bounds.origin.y)),
                            width: f64::from(f32::from(bounds.size.width)),
                            height: f64::from(f32::from(bounds.size.height)),
                        };
                        let _ = entity.update(cx, |this, cx| {
                            if measured != this.target_bounds {
                                this.target_bounds = measured;
                                if this.gaze_mode == GazeMode::Element {
                                    this.preview.update(cx, |preview, cx| {
                                        preview.remeasure_gaze_target(measured, cx)
                                    });
                                }
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
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
                    .when(self.show_target, |row| row.child(self.gaze_marker(cx)))
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
                            .child(self.gaze_controls(cx))
                            .child(self.playback_controls(cx))
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
                    .child("視線: Noneはidleへ、Restは中央、Stopは即時解除。顔測定の完全互換・エディター・保存/APIは未完了です。"),
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
