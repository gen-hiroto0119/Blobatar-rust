use blobatar_core::{Expression, Options};
use blobatar_gpui::{
    Animate, AnimatedBlobatar,
    gpui::{Context, Entity, Render, Window, div, prelude::*, px, rgb},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresenceState {
    #[default]
    Online,
    Away,
    Offline,
    Thinking,
}

impl PresenceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Online => "オンライン / online",
            Self::Away => "離席中 / away",
            Self::Offline => "オフライン / offline",
            Self::Thinking => "考え中 / thinking",
        }
    }

    fn color(self) -> u32 {
        match self {
            Self::Online => 0x42c99a,
            Self::Away => 0xf5bd64,
            Self::Offline => 0x8290a3,
            Self::Thinking => 0xa594df,
        }
    }
}

pub struct PresenceAvatar {
    name: String,
    label: Option<String>,
    state: PresenceState,
    unread: u32,
    size: f32,
    normal_expression: Expression,
    avatar: Entity<AnimatedBlobatar>,
}

impl PresenceAvatar {
    pub fn new(
        name: impl Into<String>,
        state: PresenceState,
        unread: u32,
        options: &Options,
        size: f32,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = name.into();
        let mut shown = options.clone();
        if state == PresenceState::Thinking {
            shown.expression = Some(Expression::Thinking);
        }
        let avatar = cx.new(|cx| {
            let mut avatar = AnimatedBlobatar::new(&name, &shown).size(size * 1.18);
            avatar.set_animate(Animate::Always, cx);
            avatar
        });
        Self {
            name,
            label: None,
            state,
            unread,
            size,
            normal_expression: options.expression.unwrap_or_default(),
            avatar,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn state(&self) -> PresenceState {
        self.state
    }

    pub fn set_state(&mut self, state: PresenceState, cx: &mut Context<Self>) {
        self.state = state;
        let expression = if state == PresenceState::Thinking {
            Expression::Thinking
        } else {
            self.normal_expression
        };
        self.avatar
            .update(cx, |avatar, cx| avatar.set_expression(expression, cx));
        cx.notify();
    }

    pub fn set_unread(&mut self, unread: u32, cx: &mut Context<Self>) {
        self.unread = unread;
        cx.notify();
    }

    pub fn accessible_label(&self) -> String {
        let who = self.label.as_deref().unwrap_or(&self.name);
        if self.unread == 0 {
            format!("{who}, {}", self.state.label())
        } else {
            format!("{who}, {}, {} unread", self.state.label(), self.unread)
        }
    }

    pub fn unread_label(unread: u32) -> Option<String> {
        match unread {
            0 => None,
            1..=99 => Some(unread.to_string()),
            _ => Some("99+".into()),
        }
    }
}

impl Render for PresenceAvatar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        div()
            .size(px(size))
            .relative()
            .flex_shrink_0()
            .child(
                div()
                    .absolute()
                    .left(px(-size * 0.09))
                    .top(px(-size * 0.09))
                    .child(self.avatar.clone()),
            )
            .when(self.state != PresenceState::Thinking, |view| {
                view.child(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .size(px(size * 0.26))
                        .rounded_full()
                        .border_2()
                        .border_color(rgb(0x181e29))
                        .bg(rgb(self.state.color())),
                )
            })
            .when(self.state == PresenceState::Thinking, |view| {
                view.child(
                    div()
                        .absolute()
                        .bottom(px(-6.0))
                        .left_0()
                        .w_full()
                        .text_center()
                        .text_xs()
                        .child("•••"),
                )
            })
            .when_some(Self::unread_label(self.unread), |view, unread| {
                view.child(
                    div()
                        .absolute()
                        .bottom(px(-3.0))
                        .right(px(-5.0))
                        .px_1()
                        .min_w(px(20.0))
                        .h(px(20.0))
                        .rounded_full()
                        .bg(rgb(0x9bbce3))
                        .text_color(rgb(0x10151e))
                        .text_xs()
                        .text_center()
                        .child(unread),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::PresenceAvatar;
    #[test]
    fn badge_has_zero_and_overflow_states() {
        assert_eq!(PresenceAvatar::unread_label(0), None);
        assert_eq!(PresenceAvatar::unread_label(99).as_deref(), Some("99"));
        assert_eq!(PresenceAvatar::unread_label(100).as_deref(), Some("99+"));
    }
}
