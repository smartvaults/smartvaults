// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::db::model::GetDetailedPolicyResult;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::theme::icon::{FULLSCREEN, PLUS, RELOAD, SAVE};

#[derive(Debug, Clone)]
pub enum PoliciesMessage {
    LoadPolicies(BTreeMap<EventId, GetDetailedPolicyResult>),
    SavePolicyBackup(EventId),
    Reload,
}

#[derive(Debug, Default)]
pub struct PoliciesState {
    loading: bool,
    loaded: bool,
    policies: BTreeMap<EventId, GetDetailedPolicyResult>,
}

impl PoliciesState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for PoliciesState {
    fn title(&self) -> String {
        String::from("Policies")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.get_detailed_policies().unwrap() },
            |p| PoliciesMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Policies(msg) = message {
            match msg {
                PoliciesMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                PoliciesMessage::SavePolicyBackup(policy_id) => {
                    let path = FileDialog::new()
                        .set_title("Export policy backup")
                        .set_file_name(&format!("policy-{}.json", util::cut_event_id(policy_id)))
                        .save_file();

                    if let Some(path) = path {
                        match ctx.client.save_policy_backup(policy_id, &path) {
                            Ok(_) => log::info!("Exported policy backup to {}", path.display()),
                            Err(e) => log::error!("Impossible to create file: {e}"),
                        }
                    }
                    Command::none()
                }
                PoliciesMessage::Reload => self.load(ctx),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.policies.is_empty() {
                let add_policy_btn = button::primary_with_icon(PLUS, "Add policy")
                    .width(Length::Fixed(250.0))
                    .on_press(Message::View(Stage::AddPolicy));
                let reload_btn = button::border_with_icon(RELOAD, "Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(PoliciesMessage::Reload.into());
                content = content
                    .push(Text::new("No policies").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_policy_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let add_policy_btn = button::border_only_icon(PLUS)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::AddPolicy));
                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

                if !self.loading {
                    reload_btn = reload_btn.on_press(PoliciesMessage::Reload.into());
                }

                content = content
                    .push(
                        Row::new()
                            .push(
                                Text::new("ID")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(115.0))
                                    .view(),
                            )
                            .push(Text::new("Name").bold().bigger().width(Length::Fill).view())
                            .push(
                                Text::new("Description")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Balance")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(125.0))
                                    .view(),
                            )
                            .push(add_policy_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (
                    policy_id,
                    GetDetailedPolicyResult {
                        policy,
                        balance,
                        last_sync,
                    },
                ) in self.policies.iter()
                {
                    let balance: String = if last_sync.is_some() {
                        match balance {
                            Some(balance) => {
                                format!("{} sat", util::format::big_number(balance.get_total()))
                            }
                            None => String::from("Unavailabe"),
                        }
                    } else {
                        String::from("Loading...")
                    };

                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(Text::new(&policy.name).width(Length::Fill).view())
                        .push(Text::new(&policy.description).width(Length::Fill).view())
                        .push(Text::new(balance).width(Length::Fixed(125.0)).view())
                        .push(
                            button::border_only_icon(SAVE)
                                .on_press(PoliciesMessage::SavePolicyBackup(*policy_id).into())
                                .width(Length::Fixed(40.0)),
                        )
                        .push(
                            button::primary_only_icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .width(Length::Fixed(40.0)),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<PoliciesState> for Box<dyn State> {
    fn from(s: PoliciesState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PoliciesMessage> for Message {
    fn from(msg: PoliciesMessage) -> Self {
        Self::Policies(msg)
    }
}
