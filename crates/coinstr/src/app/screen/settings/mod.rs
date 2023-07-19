// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Command, Element, Length};
use iced_aw::{Card, Modal};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::{BROADCAST_PIN, KEY, NETWORK, SETTING, TRASH};

pub mod add_relay;
pub mod change_password;
pub mod config;
pub mod relays;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    RebroadcastAllEvents,
    AskClearCache,
    CloseModal,
    ClearCache,
}

#[derive(Debug, Default)]
pub struct SettingsState {
    show_modal: bool,
}

impl SettingsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SettingsState {
    fn title(&self) -> String {
        String::from("Settings")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Settings(msg) = message {
            match msg {
                SettingsMessage::RebroadcastAllEvents => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.rebroadcast_all_events().await.unwrap() },
                        move |_| Message::View(Stage::Dashboard),
                    );
                }
                SettingsMessage::AskClearCache => self.show_modal = true,
                SettingsMessage::CloseModal => self.show_modal = false,
                SettingsMessage::ClearCache => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.clear_cache().await.unwrap() },
                        move |_| Message::View(Stage::Dashboard),
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = Column::new()
            .push(
                Button::new()
                    .text("Change password")
                    .icon(KEY)
                    .on_press(Message::View(Stage::ChangePassword))
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Config")
                    .icon(SETTING)
                    .on_press(Message::View(Stage::Config))
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Relays")
                    .icon(NETWORK)
                    .on_press(Message::View(Stage::Relays))
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Rebroadcast all events")
                    .icon(BROADCAST_PIN)
                    .on_press(SettingsMessage::RebroadcastAllEvents.into())
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Clear DB (USE ONLY IF STRICTLY NECESSARY)")
                    .icon(TRASH)
                    .style(ButtonStyle::BorderedDanger)
                    .on_press(SettingsMessage::AskClearCache.into())
                    .width(Length::Fill)
                    .view(),
            )
            .spacing(10)
            .max_width(450.0);
        let dashboard = Dashboard::new().view(ctx, content, true, true);

        Modal::new(self.show_modal, dashboard, || {
            Card::new(
                Text::new("Clear DB").view(),
                Text::new("Do you want really delete all data store into the DB?").view(),
            )
            .foot(
                Row::new()
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill)
                    .push(
                        Button::new()
                            .style(ButtonStyle::BorderedDanger)
                            .text("Confirm")
                            .width(Length::Fill)
                            .on_press(SettingsMessage::ClearCache.into())
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Close")
                            .width(Length::Fill)
                            .on_press(SettingsMessage::CloseModal.into())
                            .view(),
                    ),
            )
            .max_width(300.0)
            .into()
        })
        .on_esc(SettingsMessage::CloseModal.into())
        .backdrop(SettingsMessage::CloseModal.into())
        .into()
    }
}

impl From<SettingsState> for Box<dyn State> {
    fn from(s: SettingsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SettingsMessage> for Message {
    fn from(msg: SettingsMessage) -> Self {
        Self::Settings(msg)
    }
}
