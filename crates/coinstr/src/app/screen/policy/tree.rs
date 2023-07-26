// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::descriptor::policy::SatisfiableItem;
use coinstr_sdk::db::model::GetPolicy;
use coinstr_sdk::nostr::EventId;
use iced::widget::Column;
use iced::{Command, Element};

use crate::app::component::{Dashboard, PolicyTree};
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;

#[derive(Debug, Clone)]
pub enum PolicyTreeMessage {
    Load(SatisfiableItem),
}

#[derive(Debug)]
pub struct PolicyTreeState {
    policy_id: EventId,
    item: Option<SatisfiableItem>,
    loaded: bool,
    loading: bool,
}

impl PolicyTreeState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            policy_id,
            item: None,
            loaded: false,
            loading: false,
        }
    }
}

impl State for PolicyTreeState {
    fn title(&self) -> String {
        String::from("Policy tree")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        let policy_id = self.policy_id;
        Command::perform(
            async move {
                let GetPolicy { policy, .. } = client.get_policy_by_id(policy_id)?;
                let item = policy.satisfiable_item(client.network())?;
                Ok::<SatisfiableItem, Box<dyn std::error::Error>>(item)
            },
            |res| match res {
                Ok(item) => PolicyTreeMessage::Load(item).into(),
                Err(e) => {
                    log::error!("Impossible to load policy tree: {e}");
                    Message::View(Stage::Policies)
                }
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::PolicyTree(msg) = message {
            match msg {
                PolicyTreeMessage::Load(item) => {
                    self.item = Some(item);
                    self.loading = false;
                    self.loaded = true;
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut center_x = true;
        let mut center_y = true;

        let content = if let Some(item) = self.item.clone() {
            center_x = false;
            center_y = false;
            PolicyTree::new(item).view()
        } else {
            Column::new().push(Text::new("Tree not loaded").view())
        };

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, center_x, center_y)
    }
}

impl From<PolicyTreeState> for Box<dyn State> {
    fn from(s: PolicyTreeState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PolicyTreeMessage> for Message {
    fn from(msg: PolicyTreeMessage) -> Self {
        Self::PolicyTree(msg)
    }
}
