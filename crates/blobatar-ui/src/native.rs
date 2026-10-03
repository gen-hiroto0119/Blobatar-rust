use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use blobatar_core::{Avatar, Background, Expression};
use blobatar_export::{Motion, Settings, save_atomic};
use blobatar_gpui::{
    Animate, AnimatedBlobatar, Blobatar, Drawing,
    gpui::{
        self, AnyElement, Bounds, ClipboardItem, Context, Entity, KeyDownEvent, MouseButton,
        MouseDownEvent, MouseMoveEvent, Pixels, Render, SharedString, Subscription, Window, canvas,
        div, prelude::*, px, relative, rgb,
    },
};
use rand::Rng;

use crate::{
    axes::{self, Axis, Choice, Group, Kind, SHAPES, TONES},
    editor::EditorState,
    snippet::{self, Api},
    text_input::TextInput,
};

const ENDPOINT: &str = "http://127.0.0.1:3000/avatar/";
const CROWD: [&str; 7] = ["ひろと", "Alex", "Samira", "María", "李明", "Noor", "Kai"];

#[derive(Clone, Copy)]
enum Export {
    Svg,
    Png,
    Settings,
}

pub struct Editor {
    state: EditorState,
    name: Entity<TextInput>,
    advanced: Entity<TextInput>,
    _name_subscription: Subscription,
    preview: Entity<AnimatedBlobatar>,
    crowd: Vec<Arc<Drawing>>,
    fit: BTreeMap<String, f64>,
    axis_bounds: BTreeMap<&'static str, Bounds<Pixels>>,
    dragging: Option<&'static Axis>,
    api: Api,
    code: String,
    status: String,
    busy: bool,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = EditorState::default();
        let name = cx.new(|cx| TextInput::new(state.settings.name.clone(), cx));
        let advanced = cx.new(|cx| TextInput::new("{}", cx));
        let preview =
            cx.new(|_| AnimatedBlobatar::new(state.seed(), &state.settings.options).size(192.0));
        let subscription = cx.observe(&name, |this, input, cx| {
            let name = input.read(cx).text().to_owned();
            if name != this.state.settings.name {
                this.state.settings.name = name;
                this.refresh(true, cx);
            }
        });
        let mut editor = Self {
            state,
            name,
            advanced,
            _name_subscription: subscription,
            preview,
            crowd: Vec::new(),
            fit: BTreeMap::new(),
            axis_bounds: BTreeMap::new(),
            dragging: None,
            api: Api::Rust,
            code: String::new(),
            status: String::new(),
            busy: false,
        };
        editor.refresh(true, cx);
        editor
    }

    fn refresh(&mut self, geometry_changed: bool, cx: &mut Context<Self>) {
        let mode = match self.state.settings.motion {
            Motion::Off => Animate::Never,
            Motion::Hover => Animate::Hover,
            Motion::Always => Animate::Always,
        };
        if geometry_changed {
            self.preview = cx.new(|_| {
                AnimatedBlobatar::new(self.state.seed(), &self.state.settings.options).size(192.0)
            });
        }
        self.preview.update(cx, |preview, cx| {
            preview.set_animate(mode, cx);
            preview.set_expression(
                self.state.settings.options.expression.unwrap_or_default(),
                cx,
            );
            preview.set_reduced_motion(self.state.settings.reduced_motion, cx);
        });
        self.crowd = CROWD
            .iter()
            .map(|name| {
                Arc::new(Drawing::new(
                    &Avatar::new(name, &self.state.settings.options),
                    &self.state.settings.options,
                ))
            })
            .collect();
        self.fit = self.state.fit_readback();
        self.code = snippet::snippet(self.api, &self.state, ENDPOINT);
        let traits =
            serde_json::to_string(&self.state.settings.options.traits).expect("valid traits");
        self.advanced
            .update(cx, |input, cx| input.set_text(traits, cx));
        cx.notify();
    }

    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let action = Arc::new(action);
        let key_action = action.clone();
        div()
            .id(id.into())
            .focusable()
            .tab_stop(true)
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(if selected { 0x6d91bd } else { 0x323c4c }))
            .bg(rgb(if selected { 0x354a70 } else { 0x222938 }))
            .text_sm()
            .cursor_pointer()
            .focus(|style| style.border_color(rgb(0xf4c76b)))
            .child(label.into())
            .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    key_action(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }

    fn picker(
        &self,
        key: &'static str,
        choices: &'static [Choice],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chosen = self.state.chosen(key);
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(self.button(
                format!("{key}-auto"),
                "自動 / Auto",
                chosen.is_empty(),
                cx,
                move |this, _, cx| {
                    this.state.set_pin(key, None);
                    this.refresh(true, cx);
                },
            ))
            .child(self.button(
                format!("{key}-all"),
                "全選択 / All",
                chosen.len() == choices.len(),
                cx,
                move |this, _, cx| {
                    this.state.set_pin(
                        key,
                        axes::narrow_pin(choices.iter().map(|choice| choice.at).collect()),
                    );
                    this.refresh(true, cx);
                },
            ))
            .children(choices.iter().map(|choice| {
                self.button(
                    format!("{key}-{}", choice.name),
                    choice.name,
                    chosen.contains(&choice.at),
                    cx,
                    move |this, _, cx| {
                        this.state.toggle_choice(key, choices, choice.at);
                        this.refresh(true, cx);
                    },
                )
            }))
    }

    fn drag_axis(&mut self, axis: &'static Axis, x: Pixels, cx: &mut Context<Self>) {
        let Some(bounds) = self.axis_bounds.get(axis.key) else {
            return;
        };
        let width = f64::from(f32::from(bounds.size.width));
        if width <= 0.0 {
            return;
        }
        let value = (f64::from(f32::from(x - bounds.left())) / width).clamp(0.0, 1.0);
        self.state.pin(
            axis.key,
            axis.bands.map_or(value, |bands| {
                axes::band_value(axes::band_index(value, bands), bands)
            }),
        );
        self.refresh(true, cx);
    }

    fn slider(&self, axis: &'static Axis, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.state.reader().get(axis.key);
        let locked = self.state.settings.options.traits.contains_key(axis.key);
        let weak = cx.entity().downgrade();
        let ghost = self.fit.get(axis.key).copied();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(format!("{} · {}   {:.3}", axis.key, axis.label, value))
                    .child(self.button(
                        format!("lock-{}", axis.key),
                        if locked {
                            "解除 / Unlock"
                        } else {
                            "固定 / Lock"
                        },
                        locked,
                        cx,
                        move |this, _, cx| {
                            this.state.toggle_lock(axis.key);
                            this.refresh(true, cx);
                        },
                    )),
            )
            .child(
                div()
                    .id(axis.key)
                    .focusable()
                    .tab_stop(true)
                    .h_6()
                    .w_full()
                    .relative()
                    .rounded_md()
                    .cursor_pointer()
                    .border_1()
                    .border_color(rgb(0x323c4c))
                    .bg(rgb(0x222938))
                    .focus(|style| style.border_color(rgb(0xf4c76b)))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(relative(value as f32))
                            .rounded_md()
                            .bg(rgb(if locked { 0x627fa7 } else { 0x455469 })),
                    )
                    .when_some(ghost, |track, ghost| {
                        track.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(relative(ghost as f32))
                                .w(px(2.0))
                                .bg(rgb(0xf4c76b)),
                        )
                    })
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                let _ = weak.update(cx, |this, _| {
                                    this.axis_bounds.insert(axis.key, bounds);
                                });
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.dragging = Some(axis);
                            this.drag_axis(axis, event.position.x, cx);
                        }),
                    )
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        let value = this.state.reader().get(axis.key);
                        let next = match event.keystroke.key.as_str() {
                            "left" | "down" => {
                                axis.bands.map_or((value - 0.01).max(0.0), |bands| {
                                    axes::band_value(
                                        axes::band_index(value, bands).saturating_sub(1),
                                        bands,
                                    )
                                })
                            }
                            "right" | "up" => axis.bands.map_or((value + 0.01).min(1.0), |bands| {
                                axes::band_value(
                                    (axes::band_index(value, bands) + 1).min(bands - 1),
                                    bands,
                                )
                            }),
                            "home" => axis.bands.map_or(0.0, |bands| axes::band_value(0, bands)),
                            "end" => axis
                                .bands
                                .map_or(1.0, |bands| axes::band_value(bands - 1, bands)),
                            _ => return,
                        };
                        this.state.pin(axis.key, next);
                        this.refresh(true, cx);
                        cx.stop_propagation();
                    })),
            )
            .when_some(ghost, |row, ghost| {
                row.child(div().text_xs().text_color(rgb(0xf4c76b)).child(format!(
                    "収まり補正 / Fit: {ghost:.3} (requested {value:.3})"
                )))
            })
    }

    fn export(&mut self, format: Export, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.status = "保存先を選択 / Choose destination".into();
        let settings = self.state.settings.clone();
        let filename = match format {
            Export::Svg => "blobatar.svg",
            Export::Png => "blobatar.png",
            Export::Settings => "blobatar.json",
        };
        let prompt = cx.prompt_for_new_path(&home_directory(), Some(filename));
        cx.spawn(async move |this, cx| {
            let result = match prompt.await {
                Ok(Ok(Some(path))) => {
                    cx.background_executor()
                        .spawn(async move {
                            let bytes = match format {
                                Export::Svg => settings.svg().map(String::into_bytes),
                                Export::Png => settings.png(512),
                                Export::Settings => settings.to_json().map(String::into_bytes),
                            }?;
                            save_atomic(&path, &bytes).map_err(|error| error.to_string())?;
                            Ok(format!("保存完了 / Saved: {}", path.display()))
                        })
                        .await
                }
                Ok(Ok(None)) => Ok("取消 / Cancelled".into()),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.status =
                    result.unwrap_or_else(|error| format!("保存失敗 / Save failed: {error}"));
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.status = "設定を選択 / Choose settings".into();
        let prompt = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("設定を読み込む / Load settings".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = match prompt.await {
                Ok(Ok(Some(paths))) => {
                    cx.background_executor()
                        .spawn(async move {
                            let path = paths.first().ok_or("No path selected")?;
                            let mut json = String::new();
                            std::fs::File::open(path)
                                .map_err(|error| error.to_string())?
                                .take(1_048_577)
                                .read_to_string(&mut json)
                                .map_err(|error| error.to_string())?;
                            Settings::from_json(&json).map(Some)
                        })
                        .await
                }
                Ok(Ok(None)) => Ok(None),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(Some(settings)) => {
                        this.state.settings = settings;
                        let name = this.state.settings.name.clone();
                        this.name.update(cx, |input, cx| input.set_text(name, cx));
                        this.refresh(true, cx);
                        this.status = "読込完了 / Loaded".into();
                    }
                    Ok(None) => this.status = "取消 / Cancelled".into(),
                    Err(error) => this.status = format!("読込失敗 / Load failed: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn preview_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("editor-preview").w(px(420.0)).flex_shrink_0().h_full().overflow_y_scroll().flex().flex_col().gap_3()
            .child(div().text_sm().child("名前 / Name · 日本語IME対応"))
            .child(self.name.clone())
            .child(div().flex().gap_2()
                .child(self.button("shuffle", "名前を変更 / Shuffle", false, cx, |this, _, cx| {
                    let mut rng = rand::thread_rng();
                    let next = loop { let next = format!("{}{}", CROWD[rng.gen_range(0..CROWD.len())], rng.gen_range(10..100)); if next != this.state.settings.name { break next; } };
                    this.state.shuffle_to(next.clone()); this.name.update(cx, |input, cx| input.set_text(next, cx)); this.refresh(true, cx);
                }))
                .child(self.button("reset", "固定を解除 / Reset", false, cx, |this, _, cx| { this.state.reset(); this.refresh(true, cx); })))
            .child(div().flex().justify_center().p_4().child(self.preview.clone()))
            .child(div().text_sm().child("同じ設定・7つの名前 / Crowd"))
            .child(div().flex().gap_1().children(self.crowd.iter().enumerate().map(|(index, drawing)| {
                div().flex().flex_col().items_center().child(Blobatar::from_drawing(drawing.clone()).size(54.0)).child(div().text_xs().child(CROWD[index]))
            })))
            .child(div().flex().flex_wrap().gap_2().children(Api::ALL.into_iter().map(|api| self.button(format!("api-{}", api.name()), api.name(), self.api == api, cx, move |this, _, cx| {
                this.api = api; this.code = snippet::snippet(api, &this.state, ENDPOINT); cx.notify();
            }))))
            .child(self.button("copy-code", "コードをコピー / Copy code", false, cx, |this, _, cx| { cx.write_to_clipboard(ClipboardItem::new_string(this.code.clone())); this.status = "コピー完了 / Copied".into(); cx.notify(); }))
            .when(self.api != Api::Rust, |panel| panel.child(div().text_xs().text_color(rgb(0xf4c76b)).child("本家形式: name・traits・motionのみ。背景・表情など全設定はRust/GPUIまたは設定JSONへ。")))
            .when(self.api == Api::Http, |panel| panel.child(div().text_xs().child("自前サーバー向けのURL例です。HTTP APIはまだ未実装です。")))
            .child(div().id("editor-code").p_3().rounded_md().bg(rgb(0x151b26)).text_xs().child(self.code.clone()))
    }

    fn controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let applicable = self.state.applicable_axes();
        let background = match self.state.settings.options.background.as_ref() {
            Some(Background::Kind(kind)) => kind.as_str(),
            Some(Background::Enabled(true)) => "circle",
            _ => "none",
        };
        div()
            .id("editor-controls")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .rounded_lg()
            .bg(rgb(0x181e29))
            .child(div().text_sm().child("背景 / Background"))
            .child(
                div().flex().flex_wrap().gap_2().children(
                    ["none", "circle", "squircle", "square"]
                        .into_iter()
                        .map(|kind| {
                            self.button(
                                format!("bg-{kind}"),
                                kind,
                                background == kind,
                                cx,
                                move |this, _, cx| {
                                    this.state.settings.options.background = if kind == "none" {
                                        None
                                    } else {
                                        Some(Background::Kind(kind.into()))
                                    };
                                    this.refresh(true, cx);
                                },
                            )
                        }),
                ),
            )
            .child(div().text_sm().child("表情 / Expression"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(Expression::ALL.into_iter().map(|expression| {
                        self.button(
                            format!("expression-{}", expression.name()),
                            expression.name(),
                            self.state.settings.options.expression.unwrap_or_default()
                                == expression,
                            cx,
                            move |this, _, cx| {
                                this.state.settings.options.expression = Some(expression);
                                this.refresh(false, cx);
                            },
                        )
                    })),
            )
            .child(
                div().flex().flex_wrap().gap_2().children(
                    [
                        (Motion::Off, "停止 / Off"),
                        (Motion::Hover, "ホバー / Hover"),
                        (Motion::Always, "常時 / Always"),
                    ]
                    .into_iter()
                    .map(|(motion, label)| {
                        self.button(
                            format!("motion-{motion:?}"),
                            label,
                            self.state.settings.motion == motion,
                            cx,
                            move |this, _, cx| {
                                this.state.settings.motion = motion;
                                this.refresh(false, cx);
                            },
                        )
                    }),
                ),
            )
            .child(self.button(
                "editor-reduced-motion",
                "動きを減らす / Reduced motion",
                self.state.settings.reduced_motion,
                cx,
                |this, _, cx| {
                    this.state.settings.reduced_motion = !this.state.settings.reduced_motion;
                    this.refresh(false, cx);
                },
            ))
            .child(
                div()
                    .text_xs()
                    .child("未選択は自動。全選択は候補を保持。固定はShuffle後も保持します。"),
            )
            .children(Group::ALL.into_iter().map(|group| {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_lg().child(group.label()))
                    .children(
                        applicable
                            .iter()
                            .filter(|axis| axis.group == group)
                            .map(|axis| match axis.kind {
                                Kind::Shape => self.picker("shape", &SHAPES, cx).into_any_element(),
                                Kind::Tone => self.picker("tone", &TONES, cx).into_any_element(),
                                Kind::Slider => self.slider(axis, cx).into_any_element(),
                            }),
                    )
            }))
            .child(
                div()
                    .text_sm()
                    .child("詳細traits / Advanced JSON · 全traitキーを入力できます"),
            )
            .child(self.advanced.clone())
            .child(self.button(
                "apply-advanced",
                "適用 / Apply traits",
                false,
                cx,
                |this, _, cx| {
                    let json = this.advanced.read(cx).text().to_owned();
                    match this.state.apply_traits_json(&json) {
                        Ok(()) => {
                            this.status = "適用完了 / Applied".into();
                            this.refresh(true, cx);
                        }
                        Err(error) => {
                            this.status = format!("JSONエラー / Invalid traits: {error}");
                            cx.notify();
                        }
                    }
                },
            ))
    }
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("editor")
            .size_full()
            .bg(rgb(0x10151e))
            .text_color(rgb(0xe7edf5))
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .on_key_down(|event, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                }
            })
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some(axis) = this.dragging {
                    if event.pressed_button == Some(MouseButton::Left) {
                        this.drag_axis(axis, event.position.x, cx);
                    } else {
                        this.dragging = None;
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = None),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .child(div().text_2xl().child("Blobatar · エディター / Editor"))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(self.button(
                                "save-svg",
                                "SVG保存",
                                false,
                                cx,
                                |this, _, cx| this.export(Export::Svg, cx),
                            ))
                            .child(self.button(
                                "save-png",
                                "PNG 512px",
                                false,
                                cx,
                                |this, _, cx| this.export(Export::Png, cx),
                            ))
                            .child(self.button(
                                "save-settings",
                                "設定保存 / Save",
                                false,
                                cx,
                                |this, _, cx| this.export(Export::Settings, cx),
                            ))
                            .child(self.button(
                                "load-settings",
                                "設定読込 / Load",
                                false,
                                cx,
                                |this, _, cx| this.load(cx),
                            )),
                    ),
            )
            .child(div().text_xs().child(if self.status.is_empty() {
                "生成2 / Generation 2 · SVG/PNGは静止画です · Tabと矢印キーで操作".into()
            } else {
                self.status.clone()
            }))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_6()
                    .child(self.preview_panel(cx))
                    .child(self.controls(cx)),
            )
    }
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_owned())
}
