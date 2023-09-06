// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_sdk::nostr::EventId;
use coinstr_sdk::types::GetPolicy;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, SpinnerLinear, Text};
use crate::theme::icon::{FULLSCREEN, PLUS, RELOAD, SAVE};

#[derive(Debug, Clone)]
pub enum PoliciesMessage {
    LoadPolicies(Vec<GetPolicy>),
    SavePolicyBackup(EventId),
    Reload,
}

#[derive(Debug, Default)]
pub struct PoliciesState {
    loading: bool,
    loaded: bool,
    policies: Vec<GetPolicy>,
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
        Command::perform(async move { client.get_policies().await.unwrap() }, |p| {
            PoliciesMessage::LoadPolicies(p).into()
        })
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
                        let client = ctx.client.clone();
                        return Command::perform(
                            async move { client.save_policy_backup(policy_id, &path).await },
                            move |res| match res {
                                Ok(_) => PoliciesMessage::Reload.into(),
                                Err(_e) => PoliciesMessage::Reload.into(), /* TODO: replace this with ErrorChanged */
                            },
                        );
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
                content = content
                    .push(Text::new("No policies").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .icon(PLUS)
                            .text("Add policy")
                            .width(Length::Fixed(250.0))
                            .on_press(Message::View(Stage::AddPolicy))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .icon(RELOAD)
                            .text("Reload")
                            .width(Length::Fixed(250.0))
                            .on_press(PoliciesMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                content = content
                    .push(
                        Row::new()
                            .push(Text::new("ID").bold().width(Length::Fixed(115.0)).view())
                            .push(Text::new("Name").bold().width(Length::Fill).view())
                            .push(Text::new("Description").bold().width(Length::Fill).view())
                            .push(
                                Text::new("Balance")
                                    .bold()
                                    .width(Length::Fixed(125.0))
                                    .view(),
                            )
                            .push(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .icon(PLUS)
                                    .width(Length::Fixed(40.0))
                                    .on_press(Message::View(Stage::AddPolicy))
                                    .view(),
                            )
                            .push(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .icon(RELOAD)
                                    .width(Length::Fixed(40.0))
                                    .on_press(PoliciesMessage::Reload.into())
                                    .loading(self.loading)
                                    .view(),
                            )
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for GetPolicy {
                    policy_id,
                    policy,
                    balance,
                    last_sync,
                } in self.policies.iter()
                {
                    let balance = if last_sync.is_some() {
                        let balance: String = match balance {
                            Some(balance) => {
                                format!("{} sat", util::format::big_number(balance.total()))
                            }
                            None => String::from("Unavailabe"),
                        };
                        Column::new().push(Text::new(balance).width(Length::Fixed(125.0)).view())
                    } else {
                        Column::new().push(
                            SpinnerLinear::new()
                                .width(Length::Fixed(125.0))
                                .cycle_duration(Duration::from_secs(2)),
                        )
                    };

                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(Text::new(&policy.name).width(Length::Fill).view())
                        .push(Text::new(&policy.description).width(Length::Fill).view())
                        .push(balance)
                        .push(
                            Button::new()
                                .style(ButtonStyle::Bordered)
                                .icon(SAVE)
                                .on_press(PoliciesMessage::SavePolicyBackup(*policy_id).into())
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .width(Length::Fixed(40.0))
                                .view(),
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
