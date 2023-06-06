// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::db::model::GetNotificationsResult;
use coinstr_sdk::{util, Notification};
use iced::widget::{Column, Row};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Text};
use crate::constants::APP_NAME;
use crate::theme::color::GREY;

#[derive(Debug, Clone)]
pub enum NotificationsMessage {
    LoadNotifications(Vec<GetNotificationsResult>),
    Reload,
}

#[derive(Debug, Default)]
pub struct NotificationsState {
    loading: bool,
    loaded: bool,
    notifications: Vec<GetNotificationsResult>,
}

impl NotificationsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for NotificationsState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Notifications")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        self.loading = true;
        Command::perform(
            async move { client.db.get_notifications() },
            |res| match res {
                Ok(list) => NotificationsMessage::LoadNotifications(list).into(),
                Err(e) => {
                    log::error!("Impossible to load notifications: {e}");
                    Message::View(Stage::Dashboard)
                }
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Notifications(msg) = message {
            match msg {
                NotificationsMessage::LoadNotifications(list) => {
                    self.notifications = list;
                    self.loading = false;
                    self.loaded = true;
                }
                NotificationsMessage::Reload => {
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        let mut center_y = true;
        let mut center_x = true;

        if self.loaded {
            center_y = false;
            center_x = false;

            content = content
                .push(
                    Row::new()
                        .push(
                            Text::new("Date/Time")
                                .bold()
                                .bigger()
                                .width(Length::Fixed(225.0))
                                .view(),
                        )
                        .push(
                            Text::new("Description")
                                .bold()
                                .bigger()
                                .width(Length::Fill)
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .push(rule::horizontal_bold())
                .width(Length::Fill)
                .spacing(10);

            if self.notifications.is_empty() {
                content = content.push(Text::new("No notifications").extra_light().view());
            } else {
                for GetNotificationsResult {
                    notification,
                    timestamp,
                    seen,
                } in self.notifications.iter()
                {
                    let mut datetime =
                        Text::new(timestamp.to_human_datetime()).width(Length::Fixed(225.0));

                    let mut description = Text::new(match notification {
                        Notification::NewPolicy(id) => {
                            format!("New policy: #{}", util::cut_event_id(*id))
                        }
                        Notification::NewProposal(id) => {
                            format!("New proposal: #{}", util::cut_event_id(*id))
                        }
                    });

                    if *seen {
                        datetime = datetime.color(GREY).extra_light();
                        description = description.color(GREY).extra_light();
                    } else {
                        datetime = datetime.bold();
                        description = description.bold();
                    }

                    content = content
                        .push(
                            Row::new()
                                .push(datetime.view())
                                .push(
                                    description
                                        .on_press(match notification {
                                            Notification::NewPolicy(id) => {
                                                Message::View(Stage::Policy(*id))
                                            }
                                            Notification::NewProposal(id) => {
                                                Message::View(Stage::Proposal(*id))
                                            }
                                        })
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                        .push(rule::horizontal());
                }
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<NotificationsState> for Box<dyn State> {
    fn from(s: NotificationsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<NotificationsMessage> for Message {
    fn from(msg: NotificationsMessage) -> Self {
        Self::Notifications(msg)
    }
}