// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum WipeKeysMessage {
    PasswordChanged(String),
    Confirm,
    ErrorChanged(Option<String>),
}

#[derive(Debug, Default)]
pub struct WipeKeysState {
    password: String,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl WipeKeysState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for WipeKeysState {
    fn title(&self) -> String {
        String::from("Wipe Keys")
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        self.loaded = true;
        Command::none()
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::WipeKeys(msg) = message {
            match msg {
                WipeKeysMessage::PasswordChanged(password) => self.password = password,
                WipeKeysMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                WipeKeysMessage::Confirm => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let password = self.password.clone();
                    return Command::perform(
                        async move { client.wipe(password) },
                        |res| match res {
                            Ok(_) => Message::Lock,
                            Err(e) => WipeKeysMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        };

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = Column::new()
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .push(
                Column::new()
                    .push(Text::new("Wipe keys").big().bold().view())
                    .push(
                        Text::new("This action is permanent so make sure to have stored the keys offline, in a secure place.")
                            .extra_light()
                            .view(),
                    )
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(Space::with_height(Length::Fixed(5.0)))
            .push(
                TextInput::with_label("Password", &self.password)
                    .placeholder("Password")
                    .on_input(|p| WipeKeysMessage::PasswordChanged(p).into())
                    .on_submit(WipeKeysMessage::Confirm.into())
                    .password()
                    .view(),
            )
            .push(if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            })
            .push(
                Button::new()
                    .text("Confirm")
                    .style(ButtonStyle::Danger)
                    .on_press(WipeKeysMessage::Confirm.into())
                    .width(Length::Fill)
                    .view(),
            )
            .max_width(400.0);

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
    }
}

impl From<WipeKeysState> for Box<dyn State> {
    fn from(s: WipeKeysState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<WipeKeysMessage> for Message {
    fn from(msg: WipeKeysMessage) -> Self {
        Self::WipeKeys(msg)
    }
}
