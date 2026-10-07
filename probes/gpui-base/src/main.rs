use std::{borrow::Cow, sync::Arc};

use blobatar_core::{Avatar, Generation, Options};
use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, Focusable, Image, ImageFormat, IntoElement,
    MouseButton, Render, Subscription, Window, WindowOptions, div, img, prelude::*, px, rgb,
};
use gpui_base::{
    Button, Input, InputBase, Root,
    input::{InputEvent, InputState},
};

const FONT: &str = "Noto Sans JP";

fn main() {
    #[cfg(target_family = "wasm")]
    let app = {
        gpui_platform::web_init();
        gpui_platform::single_threaded_web()
    };
    #[cfg(not(target_family = "wasm"))]
    let app = gpui_platform::application();

    app.run(|cx: &mut App| {
        gpui_base::init(cx);
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../crates/blobatar-ui/assets/fonts/NotoSansJP.ttf")
                    .as_slice(),
            )])
            .expect("load bundled font");
        cx.open_window(WindowOptions::default(), |window, cx| {
            let probe = cx.new(|cx| Probe::new(window, cx));
            cx.new(|cx| Root::new(probe, window, cx).font_family(FONT))
        })
        .expect("open probe window");
        #[cfg(not(target_family = "wasm"))]
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

struct Probe {
    name: Entity<InputState>,
    generation: Generation,
    image: Arc<Image>,
    svg: String,
    applied_name: String,
    activations: usize,
    _input_events: Subscription,
}

impl Probe {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).default_value("blobatar"));
        name.update(cx, |state, cx| state.focus(window, cx));
        let input_events = cx.subscribe(&name, |_, _, _: &InputEvent, cx| cx.notify());
        let svg = avatar_svg("blobatar", Generation::Two);
        Self {
            name,
            generation: Generation::Two,
            image: Arc::new(Image::from_bytes(ImageFormat::Svg, svg.as_bytes().to_vec())),
            svg,
            applied_name: "blobatar".to_owned(),
            activations: 0,
            _input_events: input_events,
        }
    }

    fn generate(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value();
        self.applied_name = seed(&name).to_owned();
        self.svg = avatar_svg(&self.applied_name, self.generation);
        self.image = Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            self.svg.as_bytes().to_vec(),
        ));
        self.activations += 1;
        cx.notify();
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = self.name.clone();
        let generation = if self.generation == Generation::One {
            1
        } else {
            2
        };
        div()
            .id("probe")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(0xf0f0ee))
            .text_color(rgb(0x1d1c1a))
            .font_family(FONT)
            .p_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .max_w(px(680.))
                    .mx_auto()
                    .child(div().text_2xl().child("Blobatar / GPUI Base"))
                    .child("P0 技術検証 / Compatibility probe — not the full editor")
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_6()
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xdddcda))
                            .rounded(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .justify_center()
                                    .child(img(self.image.clone()).size(px(180.))),
                            )
                            .child(format!(
                                "適用中 / Applied: {} · Generation {generation}",
                                self.applied_name
                            ))
                            .child("名前 / Name — 空欄は blobatar / Empty uses blobatar")
                            .child(
                                InputBase::new("name-frame")
                                    .accessibility_label("名前 / Name")
                                    .focused(self.name.read(cx).focus_handle(cx).is_focused(window))
                                    .h(px(40.))
                                    .w_full()
                                    .px_3()
                                    .border_1()
                                    .border_color(rgb(0xdddcda))
                                    .rounded(px(8.))
                                    .styles(|styles| {
                                        styles.focused(|style| style.border_color(rgb(0xef551a)))
                                    })
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        input.update(cx, |state, cx| state.focus(window, cx));
                                    })
                                    .child(Input::new(&self.name)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        control("generate", "生成する / Generate").on_click(
                                            cx.listener(|this, _, _, cx| this.generate(cx)),
                                        ),
                                    )
                                    .child(control("generation", "世代切替 / Generation").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.generation = if this.generation == Generation::One
                                            {
                                                Generation::Two
                                            } else {
                                                Generation::One
                                            };
                                            this.generate(cx);
                                        }),
                                    ))
                                    .child(control("copy", "SVGをコピー / Copy SVG").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                this.svg.clone(),
                                            ));
                                        }),
                                    )),
                            )
                            .child(format!("生成回数 / Activations: {}", self.activations)),
                    )
                    .child("Tab / Shift+Tab · Enter / Space · Select / Copy / Paste")
                    .child("保存・壁・アニメーションは未接続 / No persistence, wall or animation"),
            )
    }
}

fn control(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .accessibility_label(label)
        .h(px(36.))
        .px_3()
        .flex()
        .items_center()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(0xdddcda))
        .bg(rgb(0xffffff))
        .text_color(rgb(0x1d1c1a))
        .hover(|style| style.bg(rgb(0xf0f0ee)))
        .focus(|style| style.border_color(rgb(0xef551a)))
        .child(label)
}

fn seed(name: &str) -> &str {
    if name.is_empty() { "blobatar" } else { name }
}

fn avatar_svg(name: &str, generation: Generation) -> String {
    let options = Options::default();
    Avatar::with_generation(seed(name), &options, generation).svg(&options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_uses_the_existing_editor_seed_in_both_generations() {
        for generation in [Generation::One, Generation::Two] {
            assert_eq!(
                avatar_svg("", generation),
                avatar_svg("blobatar", generation)
            );
        }
    }
}
