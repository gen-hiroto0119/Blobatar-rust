use blobatar_core::Options;
use blobatar_gpui::gpui::{
    AnyElement, Context, Entity, Image, ImageFormat, ImageSource, KeyDownEvent, Render,
    Subscription, Window, div, prelude::*, px, rgb,
};
use blobatar_ui::components::{
    Agent, AgentList, AgentSelected, ChatMessage, GroupChat, PasswordField, PresenceAvatar,
    PresenceState, ProfileAvatar, User, UserTable,
};
use std::{path::PathBuf, sync::Arc};

pub struct Components {
    profiles: Vec<Entity<ProfileAvatar>>,
    presence: Vec<Entity<PresenceAvatar>>,
    agents: Entity<AgentList>,
    users: Entity<UserTable>,
    chat: Entity<GroupChat>,
    password: Entity<PasswordField>,
    status: String,
    empty: bool,
    _selection: Subscription,
}

impl Components {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let options = Options::default();
        let portrait = ImageSource::Image(Arc::new(Image::from_bytes(ImageFormat::Svg, br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><rect width="80" height="80" fill="#a7c7e7"/><circle cx="40" cy="29" r="15" fill="#425976"/><path d="M12 80v-9a28 28 0 0 1 56 0v9" fill="#425976"/></svg>"##.to_vec())));
        let profiles = [
            Some(portrait),
            None,
            Some(ImageSource::from(PathBuf::from(
                "/blobatar-demo-intentionally-missing.png",
            ))),
        ]
        .into_iter()
        .map(|source| cx.new(|_| ProfileAvatar::new("sample-profile", source, &options, 64.0)))
        .collect();
        let presence = [
            (PresenceState::Online, 0),
            (PresenceState::Away, 3),
            (PresenceState::Offline, 99),
            (PresenceState::Thinking, 128),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (state, unread))| {
            cx.new(|cx| {
                PresenceAvatar::new(format!("member-{index}"), state, unread, &options, 52.0, cx)
            })
        })
        .collect();
        let agents = cx.new(|cx| AgentList::new(sample_agents(), options.clone(), cx));
        let selection = cx.subscribe(&agents, |this, _, selected: &AgentSelected, cx| {
            this.status = format!("選択 / Selected: {}", selected.0.name);
            cx.notify();
        });
        let users = cx.new(|_| UserTable::new(sample_users(10_000), options.clone()));
        let chat = cx.new(|cx| {
            let mut chat = GroupChat::new(sample_messages(), options.clone(), cx);
            chat.channel = Some("native-demo".into());
            chat.set_members(vec!["Alice".into(), "ぼぶ".into(), "Carla".into()], 4, cx);
            chat.set_typing(Some("Carla".into()), cx);
            chat
        });
        let password = cx.new(|cx| PasswordField::new("alain00", &options, window, cx));
        Self {
            profiles,
            presence,
            agents,
            users,
            chat,
            password,
            status: "サンプルデータのみ / Sample data — no agent or chat service connection".into(),
            empty: false,
            _selection: selection,
        }
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let action = Arc::new(action);
        let key = action.clone();
        div()
            .id(id)
            .focusable()
            .tab_stop(true)
            .px_3()
            .py_2()
            .text_sm()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x323c4c))
            .bg(rgb(0x222938))
            .cursor_pointer()
            .focus(|style| style.border_color(rgb(0xf4c76b)))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    key(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }

    fn toggle_empty(&mut self, cx: &mut Context<Self>) {
        self.empty = !self.empty;
        self.agents.update(cx, |list, cx| {
            list.set_agents(
                if self.empty {
                    Vec::new()
                } else {
                    sample_agents()
                },
                cx,
            )
        });
        self.users.update(cx, |table, cx| {
            table.set_users(
                if self.empty {
                    Vec::new()
                } else {
                    sample_users(10_000)
                },
                cx,
            )
        });
        self.chat.update(cx, |chat, cx| {
            chat.set_messages(
                if self.empty {
                    Vec::new()
                } else {
                    sample_messages()
                },
                cx,
            );
            chat.set_typing((!self.empty).then(|| "Carla".into()), cx);
        });
        cx.notify();
    }
}

impl Render for Components {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("components-gallery").size_full().overflow_y_scroll().p_6().bg(rgb(0x10151e)).text_color(rgb(0xe7edf5)).flex().flex_col().gap_6()
            .on_key_down(|event, window, cx| { if event.keystroke.key == "tab" { if event.keystroke.modifiers.shift { window.focus_prev(); } else { window.focus_next(); } cx.stop_propagation(); } })
            .child(div().text_2xl().child("再利用部品 / Components"))
            .child(div().text_sm().child(self.status.clone()))
            .child(self.button("empty-components", if self.empty { "サンプルを復元 / Restore sample" } else { "空の一覧を確認 / Empty state" }, cx, |this, _, cx| this.toggle_empty(cx)))
            .child(div().text_lg().child("画像とフォールバック / Profile avatar"))
            .child(div().flex().gap_6().children(self.profiles.iter().zip(["画像あり / Image", "未指定 / Missing", "読込失敗 / Failed"]).map(|(profile, label)| div().flex().flex_col().gap_2().items_center().child(profile.clone()).child(div().text_xs().child(label)))))
            .child(div().text_lg().child("状態と未読数 / Presence avatar"))
            .child(self.button("cycle-presence", "状態を切り替え / Cycle presence", cx, |this, _, cx| {
                for presence in &this.presence { presence.update(cx, |presence, cx| {
                    let next = match presence.state() { PresenceState::Online => PresenceState::Away, PresenceState::Away => PresenceState::Offline, PresenceState::Offline => PresenceState::Thinking, PresenceState::Thinking => PresenceState::Online };
                    presence.set_state(next, cx);
                }); }
                cx.notify();
            }))
            .child(div().flex().flex_wrap().gap_6().children(self.presence.iter().map(|presence| div().flex().flex_col().gap_3().items_center().child(presence.clone()).child(div().text_xs().child(presence.read(cx).state().label())))))
            .child(div().flex().gap_6().items_start()
                .child(div().w(px(330.0)).flex_shrink_0().child(self.agents.clone()))
                .child(div().flex_1().min_w_0().child(self.chat.clone())))
            .child(div().text_lg().child("静止アバター・10,000行 / Virtualized user table"))
            .child(div().text_xs().child("アバターの種はID、表示はname。表にTabで移動し、Home/End・矢印・Page Up/Downで操作できます。"))
            .child(self.button("rename-user", "表示名だけ変更 / Rename first user", cx, |this, _, cx| {
                let mut users = sample_users(if this.empty { 0 } else { 10_000 });
                if let Some(user) = users.first_mut() { user.name = "変更した表示名 / Renamed".into(); }
                this.users.update(cx, |table, cx| table.set_users(users, cx));
            }))
            .child(self.users.clone())
            .child(div().text_lg().child("入力位置を見る / Password field"))
            .child(div().w(px(380.0)).child(self.password.clone()))
            .child(div().text_xs().child("デモ用の文字列だけでお試しください。入力内容は保存・ログ出力・送信しません。VoiceOverなどのOSアクセシビリティ連携は未完了です。"))
    }
}

fn sample_agents() -> Vec<Agent> {
    [
        (
            "researcher",
            "Research",
            PresenceState::Thinking,
            "資料を比較中 / Comparing sources",
        ),
        (
            "builder",
            "Builder",
            PresenceState::Online,
            "待機中 / Ready",
        ),
        (
            "reviewer",
            "Reviewer",
            PresenceState::Away,
            "レビュー待ち / Waiting for review",
        ),
    ]
    .into_iter()
    .map(|(name, title, state, status)| Agent {
        name: name.into(),
        title: Some(title.into()),
        state,
        status: Some(status.into()),
        badge: (state == PresenceState::Thinking).then(|| "2 tasks".into()),
    })
    .collect()
}

fn sample_users(count: usize) -> Vec<User> {
    (0..count)
        .map(|index| User {
            id: format!("user-{index}"),
            name: format!("User {index}"),
            email: format!("user.{index}@example.invalid"),
            role: if index % 3 == 0 { "Admin" } else { "Member" }.into(),
            status: Some(if index % 5 == 0 { "invited" } else { "active" }.into()),
            last_seen: Some("Today".into()),
        })
        .collect()
}

fn sample_messages() -> Vec<ChatMessage> {
    [
        ("Alice", "09:41", "同じ送信者の発言はひとつにまとまります。"),
        (
            "Alice",
            "09:42",
            "Consecutive messages share one avatar and heading.",
        ),
        (
            "ぼぶ",
            "09:43",
            "名前と時刻、参加者、入力中の表示もあります。",
        ),
        ("Alice", "09:44", "A later message starts a new group."),
    ]
    .into_iter()
    .map(|(name, time, text)| ChatMessage {
        name: name.into(),
        title: None,
        text: text.into(),
        time: Some(time.into()),
    })
    .collect()
}
