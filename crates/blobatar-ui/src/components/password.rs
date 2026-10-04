use crate::text_input::{InputEvent, TextInput};
use blobatar_core::{Expression, Options};
use blobatar_gpui::{
    Animate, AnimatedBlobatar, GazePoint, GazeTarget,
    gpui::{
        App, Context, Entity, EventEmitter, Focusable, KeyDownEvent, Pixels, Point, Render,
        Subscription, Window, div, prelude::*, rgb,
    },
};

/// Carries no credential data. Consumers may explicitly read `PasswordField::value`.
pub struct PasswordEdited;

pub struct PasswordField {
    pub label: String,
    input: Entity<TextInput>,
    avatar: Entity<AnimatedBlobatar>,
    shown: bool,
    focused: bool,
    caret: Option<Point<Pixels>>,
    expression: Expression,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PasswordEdited> for PasswordField {}

impl PasswordField {
    pub fn new(name: &str, options: &Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("", cx).secret());
        let avatar = cx.new(|cx| {
            let mut avatar = AnimatedBlobatar::new(name, options)
                .size(160.0)
                .gaze_travel(16.0);
            avatar.set_animate(Animate::Always, cx);
            avatar.look_at(GazeTarget::Pointer, cx);
            avatar
        });
        let edit = cx.subscribe(&input, |this, _, event, cx| match event {
            InputEvent::Edited => cx.emit(PasswordEdited),
            InputEvent::CaretMoved(point) => {
                this.caret = Some(*point);
                this.aim(cx);
            }
        });
        let handle = input.read(cx).focus_handle(cx);
        let focus = cx.on_focus(&handle, window, |this, _, cx| {
            this.focused = true;
            this.aim(cx);
        });
        let blur = cx.on_blur(&handle, window, |this, _, cx| {
            this.focused = false;
            this.aim(cx);
        });
        Self {
            label: "パスワード / Password".into(),
            input,
            avatar,
            shown: false,
            focused: false,
            caret: None,
            expression: options.expression.unwrap_or_default(),
            _subscriptions: vec![edit, focus, blur],
        }
    }

    pub fn value<'a>(&'a self, cx: &'a App) -> &'a str {
        self.input.read(cx).text()
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        let value = value.into();
        if self.input.read(cx).text() != value {
            self.input.update(cx, |input, cx| input.set_text(value, cx));
        }
    }

    fn aim(&mut self, cx: &mut Context<Self>) {
        let target = if self.shown {
            GazeTarget::Rest
        } else if self.focused {
            self.caret
                .map(|point| {
                    GazeTarget::Point(GazePoint {
                        x: f32::from(point.x) as f64,
                        y: f32::from(point.y) as f64,
                    })
                })
                .unwrap_or(GazeTarget::Rest)
        } else {
            GazeTarget::Pointer
        };
        self.avatar
            .update(cx, |avatar, cx| avatar.look_at(target, cx));
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shown = !self.shown;
        self.input
            .update(cx, |input, cx| input.set_masked(!self.shown, cx));
        let expression = if self.shown {
            Expression::Sleepy
        } else {
            self.expression
        };
        self.avatar
            .update(cx, |avatar, cx| avatar.set_expression(expression, cx));
        self.input.read(cx).focus_handle(cx).focus(window);
        self.aim(cx);
        cx.notify();
    }
}

impl Render for PasswordField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_3()
            .child(div().flex().justify_center().child(self.avatar.clone()))
            .child(div().text_sm().child(self.label.clone()))
            .child(self.input.clone())
            .child(div().id("toggle-password").focusable().tab_stop(true).p_2().rounded_md().border_1().border_color(rgb(0x323c4c)).bg(rgb(0x222938)).cursor_pointer().focus(|style| style.border_color(rgb(0xf4c76b)))
                .child(if self.shown { "隠す / Hide password" } else { "表示 / Show password" })
                .on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") { this.toggle(window, cx); cx.stop_propagation(); }
                })))
            .child(div().text_xs().child(if self.shown { "目をそらしています。画面にはパスワードが表示されています。 / Not looking; password visible." } else { "入力位置を見ています / Watching the caret. This demo never saves or sends the value." }))
    }
}
