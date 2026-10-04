use super::{PresenceAvatar, PresenceState};
use blobatar_core::Options;
use blobatar_gpui::gpui::{
    Context, ElementId, Entity, EventEmitter, KeyDownEvent, Render, SharedString, Window, div,
    prelude::*, px, rgb,
};

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: String,
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
    active_id: Option<String>,
    options: Options,
}

impl EventEmitter<AgentSelected> for AgentList {}

impl AgentList {
    pub fn new(agents: Vec<Agent>, options: Options, cx: &mut Context<Self>) -> Self {
        let mut list = Self {
            label: "エージェント / Agents".into(),
            agents: Vec::new(),
            avatars: Vec::new(),
            active_id: None,
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
        self.active_id = retained_selection(&agents, self.active_id.as_deref());
        self.agents = agents;
        cx.notify();
    }

    /// Selects an agent by its stable public ID.
    pub fn select(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        self.active_id = retained_selection(&self.agents, id.as_deref());
        cx.notify();
    }

    pub fn working_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|agent| agent.state == PresenceState::Thinking)
            .count()
    }

    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(agent) = activated_agent(&self.agents, index) else {
            return;
        };
        self.active_id = Some(agent.id.clone());
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
                let selected = is_active_agent(agent, self.active_id.as_deref());
                div()
                    .id(ElementId::from(SharedString::from(agent.id.clone())))
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

fn retained_selection(agents: &[Agent], selected_id: Option<&str>) -> Option<String> {
    selected_id
        .filter(|id| agents.iter().any(|agent| agent.id == *id))
        .map(str::to_owned)
}

fn is_active_agent(agent: &Agent, selected_id: Option<&str>) -> bool {
    selected_id == Some(agent.id.as_str())
}

fn activated_agent(agents: &[Agent], index: usize) -> Option<Agent> {
    agents.get(index).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, name: &str) -> Agent {
        Agent {
            id: id.to_owned(),
            name: name.to_owned(),
            title: None,
            state: PresenceState::Online,
            status: None,
            badge: None,
        }
    }

    #[test]
    fn duplicate_names_remain_independently_selectable_by_stable_id() {
        let first = agent("agent-a", "same name");
        let second = agent("agent-b", "same name");
        let agents = vec![first.clone(), second.clone()];

        let selected = retained_selection(&agents, Some("agent-b"));
        assert_eq!(selected.as_deref(), Some("agent-b"));
        assert!(!is_active_agent(&first, selected.as_deref()));
        assert!(is_active_agent(&second, selected.as_deref()));
        assert_eq!(activated_agent(&agents, 1).unwrap().id, "agent-b");
    }

    #[test]
    fn selection_survives_reordering_and_rename_but_clears_when_removed() {
        let first = agent("agent-a", "same name");
        let second = agent("agent-b", "same name");
        let selected = retained_selection(&[first.clone(), second.clone()], Some("agent-b"));

        let reordered = vec![second.clone(), first];
        let selected = retained_selection(&reordered, selected.as_deref());
        assert_eq!(selected.as_deref(), Some("agent-b"));

        let renamed = agent("agent-b", "renamed");
        let selected = retained_selection(&[renamed], selected.as_deref());
        assert_eq!(selected.as_deref(), Some("agent-b"));

        assert_eq!(retained_selection(&[], selected.as_deref()), None);
    }
}
