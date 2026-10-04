use std::{
    collections::BTreeMap,
    io::Read,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use blobatar_core::{Avatar, Background, Expression};
use blobatar_export::{Motion, Settings, save_atomic};
use blobatar_gpui::{
    Animate, AnimatedBlobatar, Blobatar, Drawing,
    gpui::{
        self, AnyElement, Bounds, ClipboardItem, Context, Entity, FontWeight, HighlightStyle,
        KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Render, SharedString,
        StyledText, Subscription, Window, canvas, div, prelude::*, px, relative, rgb,
    },
};
use rand::Rng;

use crate::{
    advanced_json::AdvancedJsonDraft,
    axes::{self, Axis, Choice, Group, Kind, TONES},
    code_highlight::rust_highlights,
    editor::EditorState,
    snippet,
    text_input::TextInput,
    theme::{self, Appearance, ButtonStyle},
};

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
    advanced_json: AdvancedJsonDraft,
    _name_subscription: Subscription,
    preview: Entity<AnimatedBlobatar>,
    crowd: Vec<Arc<Drawing>>,
    fit: BTreeMap<String, f64>,
    axis_bounds: BTreeMap<&'static str, Bounds<Pixels>>,
    dragging: Option<&'static Axis>,
    code: String,
    code_highlights: Vec<(Range<usize>, HighlightStyle)>,
    status: String,
    busy: bool,
    appearance: Appearance,
    copied: bool,
}

impl Editor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = EditorState::default();
        let name = cx.new(|cx| TextInput::new(state.settings.name.clone(), cx));
        let advanced_text =
            serde_json::to_string(&state.settings.options.traits).expect("valid traits");
        let advanced = cx.new(|cx| TextInput::new(advanced_text.clone(), cx));
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
            advanced_json: AdvancedJsonDraft::new(advanced_text),
            _name_subscription: subscription,
            preview,
            crowd: Vec::new(),
            fit: BTreeMap::new(),
            axis_bounds: BTreeMap::new(),
            dragging: None,
            code: String::new(),
            code_highlights: Vec::new(),
            status: String::new(),
            busy: false,
            appearance: Appearance::default(),
            copied: false,
        };
        editor.sync_appearance(cx);
        editor.refresh(true, cx);
        editor
    }

    fn refresh(&mut self, geometry_changed: bool, cx: &mut Context<Self>) {
        self.refresh_with_advanced_sync(geometry_changed, false, cx);
    }

    fn refresh_with_advanced_sync(
        &mut self,
        geometry_changed: bool,
        force_advanced_sync: bool,
        cx: &mut Context<Self>,
    ) {
        let mode = match self.state.settings.motion {
            Motion::Off => Animate::Never,
            Motion::Hover => Animate::Hover,
            Motion::Always => Animate::Always,
        };
        if geometry_changed {
            self.preview = cx.new(|_| {
                AnimatedBlobatar::with_generation(
                    self.state.seed(),
                    &self.state.settings.options,
                    self.state.settings.generation(),
                )
                .size(192.0)
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
                    &Avatar::with_generation(
                        name,
                        &self.state.settings.options,
                        self.state.settings.generation(),
                    ),
                    &self.state.settings.options,
                ))
            })
            .collect();
        self.fit = self.state.fit_readback();
        let code = snippet::snippet(&self.state);
        if self.code != code {
            self.copied = false;
        }
        self.code = code;
        self.code_highlights = rust_highlights(&self.code);
        let traits =
            serde_json::to_string(&self.state.settings.options.traits).expect("valid traits");
        let advanced = self.advanced.clone();
        let draft = &mut self.advanced_json;
        advanced.update(cx, |input, cx| {
            if force_advanced_sync {
                input.set_invalid(false, cx);
            }
            if let Some(traits) = draft.sync(input.text(), traits, force_advanced_sync) {
                input.set_text(traits, cx);
            }
        });
        cx.notify();
    }

    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        style: impl Into<ButtonStyle>,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let style = style.into();
        let action = Arc::new(action);
        let key_action = action.clone();
        theme::button(id, label, self.appearance, style)
            .on_click(cx.listener(move |this, _, window, cx| {
                if !style.disabled {
                    action(this, window, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !style.disabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    key_action(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }

    fn sync_appearance(&mut self, cx: &mut Context<Self>) {
        for input in [&self.name, &self.advanced] {
            input.update(cx, |input, cx| input.set_appearance(self.appearance, cx));
        }
        cx.notify();
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
        let palette = self.appearance.palette();
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
                    .border_color(palette.border)
                    .bg(palette.background)
                    .focus(move |style| style.border_2().border_color(palette.accent))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(relative(value as f32))
                            .rounded_md()
                            .bg(if locked {
                                palette.accent
                            } else {
                                palette.muted.opacity(0.45)
                            }),
                    )
                    .when_some(ghost, |track, ghost| {
                        track.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(relative(ghost as f32))
                                .w(px(2.0))
                                .bg(palette.selected_text),
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
                row.child(
                    div()
                        .text_xs()
                        .text_color(palette.selected_text)
                        .child(format!(
                            "収まり補正 / Fit: {ghost:.3} (requested {value:.3})"
                        )),
                )
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
                        this.refresh_with_advanced_sync(true, true, cx);
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

    fn preview_panel(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        theme::panel(self.appearance)
            .id("editor-preview")
            .min_w_0()
            .when(compact, |panel| panel.w_full())
            .when(!compact, |panel| {
                panel.w(px(420.0)).h_full().overflow_y_scroll()
            })
            .child(theme::heading("プレビュー / Preview"))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child("名前 / Name"),
            )
            .child(self.name.clone())
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.button(
                        "shuffle",
                        "名前を変更 / Shuffle",
                        false,
                        cx,
                        |this, _, cx| {
                            let mut rng = rand::thread_rng();
                            let next = loop {
                                let next = format!(
                                    "{}{}",
                                    CROWD[rng.gen_range(0..CROWD.len())],
                                    rng.gen_range(10..100)
                                );
                                if next != this.state.settings.name {
                                    break next;
                                }
                            };
                            this.state.shuffle_to(next.clone());
                            this.name.update(cx, |input, cx| input.set_text(next, cx));
                            this.refresh(true, cx);
                        },
                    ))
                    .child(self.button(
                        "reset",
                        "固定を解除 / Reset",
                        false,
                        cx,
                        |this, _, cx| {
                            this.state.reset();
                            this.refresh_with_advanced_sync(true, true, cx);
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .min_h(px(232.0))
                    .flex_shrink_0()
                    .rounded(px(theme::CONTROL_RADIUS))
                    .bg(palette.background)
                    .child(self.preview.clone()),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child("同じ設定・7つの名前 / Crowd"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(self.crowd.iter().enumerate().map(|(index, drawing)| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(Blobatar::from_drawing(drawing.clone()).size(54.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .line_height(px(18.0))
                                    .text_color(palette.muted)
                                    .child(CROWD[index]),
                            )
                    })),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(palette.border)
                    .pt_4()
                    .child(theme::heading("生成コード / Rust/GPUI")),
            )
            .child(self.button(
                "copy-code",
                if self.copied {
                    "コピー完了 / Copied"
                } else {
                    "Rustコードをコピー / Copy Rust code"
                },
                self.copied,
                cx,
                |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(this.code.clone()));
                    this.copied = true;
                    this.status = "コピー完了 / Copied".into();
                    cx.notify();
                },
            ))
            .child(
                div()
                    .id("editor-code")
                    .w_full()
                    .flex_shrink_0()
                    .overflow_x_scroll()
                    .p_4()
                    .rounded(px(theme::CONTROL_RADIUS))
                    .bg(rgb(0x1d1c1a))
                    .font(theme::font_with_japanese_fallback(theme::CODE_FONT))
                    .text_color(rgb(0xe6edf5))
                    .text_size(px(12.0))
                    .line_height(px(20.0))
                    .whitespace_nowrap()
                    .child(
                        StyledText::new(self.code.clone())
                            .with_highlights(self.code_highlights.clone()),
                    ),
            )
    }

    fn controls(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        let applicable = self.state.applicable_axes();
        let background = match self.state.settings.options.background.as_ref() {
            Some(Background::Kind(kind)) => kind.as_str(),
            Some(Background::Enabled(true)) => "circle",
            _ => "none",
        };
        theme::panel(self.appearance)
            .id("editor-controls")
            .min_w_0()
            .when(compact, |panel| panel.w_full())
            .when(!compact, |panel| panel.flex_1().h_full().overflow_y_scroll())
            .child(theme::heading("カスタマイズ / Customize"))
            .child(div().text_sm().child("世代 / Generation"))
            .child(div().flex().flex_wrap().gap_2().children([1, 2].into_iter().map(|generation| {
                self.button(
                    format!("generation-{generation}"),
                    format!("生成{generation} / Generation {generation}"),
                    self.state.settings.generation == generation,
                    cx,
                    move |this, _, cx| {
                        this.state.settings.generation = generation;
                        this.dragging = None;
                        this.status = "世代を変更しました。固定した数値は保持します / Numeric overrides are preserved".into();
                        this.refresh(true, cx);
                    },
                )
            })))
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
                    .text_color(palette.muted)
                    .child("未選択は自動。全選択は候補を保持。固定はShuffle後も保持します。"),
            )
            .children(Group::ALL.into_iter().map(|group| {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .flex_shrink_0()
                    .pt_4()
                    .border_t_1()
                    .border_color(palette.border)
                    .child(theme::heading(group.label()))
                    .children(
                        applicable
                            .iter()
                            .filter(|axis| axis.group == group)
                            .map(|axis| match axis.kind {
                                Kind::Shape => self.picker("shape", self.state.shape_choices(), cx).into_any_element(),
                                Kind::Tone => self.picker("tone", &TONES, cx).into_any_element(),
                                Kind::Slider => self.slider(axis, cx).into_any_element(),
                            }),
                    )
            }))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(palette.muted)
                    .pt_4()
                    .border_t_1()
                    .border_color(palette.border)
                    .child(
                        "詳細traits / Advanced JSON · 全traitキーを入力できます · 未適用JSONはドラフトとして保持 / Unapplied JSON stays as a draft",
                    ),
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
                            this.refresh_with_advanced_sync(true, true, cx);
                        }
                        Err(error) => {
                            this.advanced.update(cx, |input, cx| input.set_invalid(true, cx));
                            this.status = format!("JSONエラー / Invalid traits: {error}");
                            cx.notify();
                        }
                    }
                },
            ))
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        let compact = window.viewport_size().width < px(980.0);
        div()
            .id("editor")
            .size_full()
            .bg(palette.background)
            .text_color(palette.text)
            .font_family(theme::BODY_FONT)
            .text_size(px(14.0))
            .line_height(px(22.0))
            .flex()
            .flex_col()
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
                    .flex_shrink_0()
                    .h(px(48.0))
                    .px_6()
                    .bg(palette.surface)
                    .border_b_1()
                    .border_color(palette.border)
                    .child(div().font_family(theme::HEADING_FONT).font_weight(FontWeight::SEMIBOLD)
                        .child("Blobatar").child(div().ml_4().text_color(palette.muted).child("/ Editor" )).flex())
                    .child(self.button(
                        "editor-theme",
                        if self.appearance == Appearance::Light { "ダーク / Dark" } else { "ライト / Light" },
                        false,
                        cx,
                        |this, _, cx| {
                            this.appearance = this.appearance.toggled();
                            this.sync_appearance(cx);
                        },
                    )),
            )
            .child(div().flex().flex_col().flex_1().min_h_0().gap_4().p_6()
                .when(compact, |body| body.p_4())
                .child(div().flex().flex_wrap().justify_between().items_center().gap_4().flex_shrink_0()
                    .child(div().flex().flex_col().gap_1()
                        .child(div().font(theme::font_with_japanese_fallback(theme::HEADING_FONT))
                            .font_weight(FontWeight::SEMIBOLD).text_size(px(28.0)).line_height(px(36.0))
                            .child("エディター / Editor"))
                        .child(div().text_size(px(12.0)).line_height(px(18.0)).text_color(palette.muted)
                            .child("名前から、自分だけのアバターを。 / Create your avatar.")))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(self.button(
                                "save-svg",
                                "SVG保存",
                                ButtonStyle::primary(self.busy),
                                cx,
                                |this, _, cx| this.export(Export::Svg, cx),
                            ))
                            .child(self.button(
                                "save-png",
                                "PNG 512px",
                                ButtonStyle::secondary(self.busy),
                                cx,
                                |this, _, cx| this.export(Export::Png, cx),
                            ))
                            .child(self.button(
                                "save-settings",
                                "設定保存 / Save",
                                ButtonStyle::secondary(self.busy),
                                cx,
                                |this, _, cx| this.export(Export::Settings, cx),
                            ))
                            .child(self.button(
                                "load-settings",
                                "設定読込 / Load",
                                ButtonStyle::secondary(self.busy),
                                cx,
                                |this, _, cx| this.load(cx),
                            )),
                    ),
            )
            .child(div().flex_shrink_0().text_size(px(12.0)).line_height(px(18.0))
                .text_color(palette.muted).child(if self.status.is_empty() {
                format!(
                    "生成{} / Generation {} · SVG/PNGは静止画です · Tabと矢印キーで操作",
                    self.state.settings.generation, self.state.settings.generation
                )
            } else {
                self.status.clone()
            }))
            .child(
                div()
                    .id("editor-panels")
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_6()
                    .when(compact, |panels| panels.flex_col().overflow_y_scroll())
                    .child(self.preview_panel(compact, cx))
                    .child(self.controls(compact, cx)),
            ))
    }
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_owned())
}
