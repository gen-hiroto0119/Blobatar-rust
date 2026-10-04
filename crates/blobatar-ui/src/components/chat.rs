use blobatar_core::{Background, Options};
use blobatar_gpui::{
    AnimatedBlobatar, Blobatar, Drawing,
    gpui::{Context, Entity, Render, Window, div, prelude::*, px, rgb},
};
use std::{ops::Range, sync::Arc};

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub name: String,
    pub title: Option<String>,
    pub text: String,
    pub time: Option<String>,
}

pub fn message_runs(messages: &[ChatMessage]) -> Vec<Range<usize>> {
    let mut runs: Vec<Range<usize>> = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        if let Some(last) = runs.last_mut()
            && messages[last.start].name == message.name
        {
            last.end = index + 1;
            continue;
        }
        runs.push(index..index + 1);
    }
    runs
}

pub struct GroupChat {
    pub channel: Option<String>,
    pub extra_members: u32,
    messages: Vec<ChatMessage>,
    runs: Vec<(Range<usize>, Entity<AnimatedBlobatar>)>,
    members: Vec<(String, Arc<Drawing>)>,
    typing: Option<(String, Arc<Drawing>)>,
    options: Options,
}

impl GroupChat {
    pub fn new(messages: Vec<ChatMessage>, options: Options, cx: &mut Context<Self>) -> Self {
        let mut chat = Self {
            channel: None,
            extra_members: 0,
            messages: Vec::new(),
            runs: Vec::new(),
            members: Vec::new(),
            typing: None,
            options,
        };
        chat.set_messages(messages, cx);
        chat
    }

    pub fn set_messages(&mut self, messages: Vec<ChatMessage>, cx: &mut Context<Self>) {
        self.runs = message_runs(&messages)
            .into_iter()
            .map(|range| {
                let avatar = cx.new(|_| {
                    AnimatedBlobatar::new(&messages[range.start].name, &self.options).size(40.0)
                });
                (range, avatar)
            })
            .collect();
        self.messages = messages;
        cx.notify();
    }

    pub fn set_members(&mut self, members: Vec<String>, extra: u32, cx: &mut Context<Self>) {
        let mut options = self.options.clone();
        if options.background.is_none() {
            options.background = Some(Background::Kind("circle".into()));
        }
        self.members = members
            .into_iter()
            .map(|name| {
                let drawing = super::drawing(&name, &options);
                (name, drawing)
            })
            .collect();
        self.extra_members = extra;
        cx.notify();
    }

    pub fn set_typing(&mut self, name: Option<String>, cx: &mut Context<Self>) {
        self.typing = name.map(|name| {
            let drawing = super::drawing(&name, &self.options);
            (name, drawing)
        });
        cx.notify();
    }
}

impl Render for GroupChat {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("group-chat")
            .max_h(px(520.0))
            .overflow_y_scroll()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x323c4c))
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .when(self.channel.is_some() || !self.members.is_empty(), |view| {
                view.child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .flex_wrap()
                        .gap_3()
                        .child(
                            div().text_sm().child(
                                self.channel
                                    .as_ref()
                                    .map(|channel| format!("# {channel}"))
                                    .unwrap_or_default(),
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .children(self.members.iter().enumerate().map(
                                    |(index, (_, drawing))| {
                                        div().ml(px(if index == 0 { 0.0 } else { -8.0 })).child(
                                            Blobatar::from_drawing(drawing.clone()).size(28.0),
                                        )
                                    },
                                ))
                                .when(self.extra_members > 0, |members| {
                                    members.child(
                                        div().text_xs().child(format!("+{}", self.extra_members)),
                                    )
                                }),
                        ),
                )
            })
            .when(!self.members.is_empty(), |view| {
                view.child(div().text_xs().text_color(rgb(0xa9b7ca)).child(format!(
                        "参加者 / Members: {}",
                        self.members
                            .iter()
                            .map(|(name, _)| name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )))
            })
            .when(self.messages.is_empty(), |view| {
                view.child(div().text_sm().child("メッセージなし / No messages"))
            })
            .children(
                self.runs
                    .iter()
                    .enumerate()
                    .map(|(index, (range, avatar))| {
                        let first = &self.messages[range.start];
                        div()
                            .id(index)
                            .flex()
                            .gap_3()
                            .min_w_0()
                            .child(avatar.clone())
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_2()
                                            .flex_wrap()
                                            .items_center()
                                            .child(
                                                div().text_sm().child(
                                                    first
                                                        .title
                                                        .as_deref()
                                                        .unwrap_or(&first.name)
                                                        .to_owned(),
                                                ),
                                            )
                                            .when_some(first.time.clone(), |head, time| {
                                                head.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xa9b7ca))
                                                        .child(time),
                                                )
                                            }),
                                    )
                                    .children(self.messages[range.clone()].iter().map(|message| {
                                        div().text_sm().child(message.text.clone())
                                    })),
                            )
                    }),
            )
            .when_some(self.typing.as_ref(), |view, (name, drawing)| {
                view.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .opacity(0.5)
                                .child(Blobatar::from_drawing(drawing.clone()).size(40.0)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .child(format!("{name} が入力中 / is typing…")),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{ChatMessage, message_runs};
    #[test]
    fn groups_consecutive_senders_without_merging_later_runs() {
        let messages = ["a", "a", "b", "a"]
            .into_iter()
            .map(|name| ChatMessage {
                name: name.into(),
                title: None,
                text: "duplicate text".into(),
                time: None,
            })
            .collect::<Vec<_>>();
        assert_eq!(message_runs(&messages), vec![0..2, 2..3, 3..4]);
        assert!(message_runs(&[]).is_empty());
    }
}
