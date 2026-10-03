use super::{PresenceAvatar, PresenceState};
use blobatar_core::Options;
use blobatar_gpui::gpui::{
    Context, Entity, EventEmitter, KeyDownEvent, Render, Window, div, prelude::*, px, rgb,
};

#[derive(Clone, Debug)]
pub struct Agent {
    pub name: String,
    pub title: Option<String>,
    pub state: PresenceState,
    pub status: Option<String>,
    pub badge: Option<String>,
}

pub struct AgentSelected(pub Agent);

pub struct AgentList {
    pub label: String,
    agents: Vec<Agent>,
    avatars: Vec<Entity<PresenceAvatar>>,
    active: Option<String>,
    options: Options,
}

impl EventEmitter<AgentSelected> for AgentList {}

impl AgentList {
    pub fn new(agents: Vec<Agent>, options: Options, cx: &mut Context<Self>) -> Self {
        let mut list = Self {
            label: "エージェント / Agents".into(),
            agents: Vec::new(),
            avatars: Vec::new(),
            active: None,
            options,
        };
        list.set_agents(agents, cx);
        list
    }

    pub fn set_agents(&mut self, agents: Vec<Agent>, cx: &mut Context<Self>) {
        self.avatars = agents
            .iter()
            .map(|agent| {
                cx.new(|cx| {
                    PresenceAvatar::new(&agent.name, agent.state, 0, &self.options, 36.0, cx)
                        .label(agent.title.as_deref().unwrap_or(&agent.name))
                })
            })
            .collect();
        if !agents
            .iter()
            .any(|agent| Some(&agent.name) == self.active.as_ref())
        {
            self.active = None;
        }
        self.agents = agents;
        cx.notify();
    }

    pub fn select(&mut self, name: Option<String>, cx: &mut Context<Self>) {
        self.active = name.filter(|name| self.agents.iter().any(|agent| &agent.name == name));
        cx.notify();
    }

    pub fn working_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|agent| agent.state == PresenceState::Thinking)
            .count()
    }

    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        let agent = self.agents[index].clone();
        self.active = Some(agent.name.clone());
        cx.emit(AgentSelected(agent));
        cx.notify();
    }
}

impl Render for AgentList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("agents")
            .flex()
            .flex_col()
            .gap_2()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_sm()
                    .child(self.label.clone())
                    .child(format!(
                        "稼働 / working {}/{}",
                        self.working_count(),
                        self.agents.len()
                    )),
            )
            .when(self.agents.is_empty(), |view| {
                view.child(div().p_4().text_sm().child("エージェントなし / No agents"))
            })
            .children(self.agents.iter().enumerate().map(|(index, agent)| {
                let selected = self.active.as_deref() == Some(&agent.name);
                div()
                    .id(index)
                    .focusable()
                    .tab_stop(true)
                    .min_h(px(68.0))
                    .p_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .rounded_md()
                    .border_1()
                    .bg(rgb(if selected { 0x354a70 } else { 0x222938 }))
                    .border_color(rgb(0x323c4c))
                    .focus(|style| style.border_color(rgb(0xf4c76b)))
                    .cursor_pointer()
                    .child(self.avatars[index].clone())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div().text_sm().truncate().child(
                                    agent.title.as_deref().unwrap_or(&agent.name).to_owned(),
                                ),
                            )
                            .child(
                                div().text_xs().text_color(rgb(0xa9b7ca)).truncate().child(
                                    agent
                                        .status
                                        .as_deref()
                                        .unwrap_or(agent.state.label())
                                        .to_owned(),
                                ),
                            )
                            .when(agent.status.is_some(), |column| {
                                column.child(div().text_xs().child(agent.state.label()))
                            }),
                    )
                    .when_some(agent.badge.clone(), |row, badge| {
                        row.child(div().text_xs().max_w(px(70.0)).truncate().child(badge))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.activate(index, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.activate(index, cx);
                            cx.stop_propagation();
                        }
                    }))
            }))
    }
}
