use blobatar_core::{Background, Options};
use blobatar_gpui::{
    Blobatar, Drawing,
    gpui::{
        Context, KeyDownEvent, Render, ScrollStrategy, UniformListScrollHandle, Window, div,
        prelude::*, px, rgb, uniform_list,
    },
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: Option<String>,
    pub last_seen: Option<String>,
}

pub struct UserTable {
    pub caption: String,
    users: Vec<User>,
    options: Options,
    drawings: BTreeMap<String, Arc<Drawing>>,
    cache_order: VecDeque<String>,
    scroll: UniformListScrollHandle,
    keyboard_row: usize,
}

impl UserTable {
    pub fn new(users: Vec<User>, mut options: Options) -> Self {
        if options.background.is_none() {
            options.background = Some(Background::Kind("squircle".into()));
        }
        Self {
            caption: "ユーザー / Users".into(),
            users,
            options,
            drawings: BTreeMap::new(),
            cache_order: VecDeque::new(),
            scroll: UniformListScrollHandle::new(),
            keyboard_row: 0,
        }
    }

    pub fn set_users(&mut self, users: Vec<User>, cx: &mut Context<Self>) {
        self.users = users;
        self.keyboard_row = 0;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    pub fn len(&self) -> usize {
        self.users.len()
    }
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    fn drawing(&mut self, id: &str) -> Arc<Drawing> {
        if let Some(drawing) = self.drawings.get(id) {
            return drawing.clone();
        }
        if self.cache_order.len() == 256
            && let Some(oldest) = self.cache_order.pop_front()
        {
            self.drawings.remove(&oldest);
        }
        let drawing = super::drawing(id, &self.options);
        self.cache_order.push_back(id.into());
        self.drawings.insert(id.into(), drawing.clone());
        drawing
    }
}

impl Render for UserTable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("user-table")
            .focusable()
            .tab_stop(true)
            .h(px(340.0))
            .flex_shrink_0()
            .w_full()
            .overflow_x_scroll()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x323c4c))
            .focus(|style| style.border_color(rgb(0xf4c76b)))
            .child(
                div()
                    .min_w(px(660.0))
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(div().p_3().text_sm().child(format!(
                        "{} · {}",
                        self.caption,
                        self.users.len()
                    )))
                    .child(
                        div()
                            .h_8()
                            .flex_shrink_0()
                            .px_3()
                            .flex()
                            .text_xs()
                            .child(div().flex_1().child("ユーザー / User"))
                            .child(div().w(px(110.0)).child("役割 / Role"))
                            .child(div().w(px(120.0)).child("状態 / Status"))
                            .child(
                                div()
                                    .w(px(110.0))
                                    .text_right()
                                    .child("最終確認 / Last seen"),
                            ),
                    )
                    .when(self.users.is_empty(), |view| {
                        view.child(div().p_4().child("ユーザーなし / No users"))
                    })
                    .when(!self.users.is_empty(), |view| {
                        view.child(
                            uniform_list(
                                "user-rows",
                                self.users.len(),
                                cx.processor(|this, range: std::ops::Range<usize>, _, _| {
                                    range
                                        .map(|index| {
                                            let user = this.users[index].clone();
                                            let drawing = this.drawing(&user.id);
                                            div()
                                                .id(index)
                                                .h(px(64.0))
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .border_b_1()
                                                .border_color(rgb(0x323c4c))
                                                .hover(|style| style.bg(rgb(0x222938)))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .flex()
                                                        .items_center()
                                                        .gap_3()
                                                        .child(
                                                            Blobatar::from_drawing(drawing)
                                                                .size(36.0),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w_0()
                                                                .flex()
                                                                .flex_col()
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .truncate()
                                                                        .child(user.name),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .truncate()
                                                                        .text_color(rgb(0xa9b7ca))
                                                                        .child(user.email),
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(102.0))
                                                        .flex_shrink_0()
                                                        .text_sm()
                                                        .truncate()
                                                        .child(user.role),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(112.0))
                                                        .flex_shrink_0()
                                                        .text_xs()
                                                        .truncate()
                                                        .child(user.status.unwrap_or_default()),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(110.0))
                                                        .flex_shrink_0()
                                                        .text_xs()
                                                        .text_right()
                                                        .truncate()
                                                        .child(user.last_seen.unwrap_or_default()),
                                                )
                                        })
                                        .collect()
                                }),
                            )
                            .track_scroll(self.scroll.clone())
                            .flex_1()
                            .min_h_0(),
                        )
                    }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let last = this.users.len().saturating_sub(1);
                this.keyboard_row = match event.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => last,
                    "down" => (this.keyboard_row + 1).min(last),
                    "up" => this.keyboard_row.saturating_sub(1),
                    "pagedown" => (this.keyboard_row + 4).min(last),
                    "pageup" => this.keyboard_row.saturating_sub(4),
                    _ => return,
                };
                this.scroll
                    .scroll_to_item(this.keyboard_row, ScrollStrategy::Top);
                cx.notify();
                cx.stop_propagation();
            }))
    }
}
