// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum ChangePasswordMessage {
    PasswordChanged(String),
    NewPasswordChanged(String),
    ConfirmNewPasswordChanged(String),
    ErrorChanged(Option<String>),
    Save,
}

#[derive(Debug, Default)]
pub struct ChangePasswordState {
    password: String,
    new_password: String,
    confirm_new_password: String,
    loading: bool,
    error: Option<String>,
}

impl ChangePasswordState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ChangePasswordState {
    fn title(&self) -> String {
        String::from("Change password")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::ChangePassword(msg) = message {
            match msg {
                ChangePasswordMessage::PasswordChanged(password) => self.password = password,
                ChangePasswordMessage::NewPasswordChanged(new_password) => {
                    self.new_password = new_password
                }
                ChangePasswordMessage::ConfirmNewPasswordChanged(confirm_new_password) => {
                    self.confirm_new_password = confirm_new_password
                }
                ChangePasswordMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                ChangePasswordMessage::Save => {
                    let client = ctx.client.clone();
                    let password = self.password.clone();
                    let new_password = self.new_password.clone();
                    let confirm_new_password = self.confirm_new_password.clone();
                    self.loading = true;
                    return Command::perform(
                        async move {
                            client.change_password(
                                || Ok(password),
                                || Ok(new_password),
                                || Ok(confirm_new_password),
                            )
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Settings),
                            Err(e) => {
                                ChangePasswordMessage::ErrorChanged(Some(e.to_string())).into()
                            }
                        },
                    );
                }
            }
        };

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let password = TextInput::with_label("Password", &self.password)
            .on_input(|s| ChangePasswordMessage::PasswordChanged(s).into())
            .placeholder("Password")
            .password()
            .on_submit(ChangePasswordMessage::Save.into())
            .view();

        let new_password = TextInput::with_label("New password", &self.new_password)
            .on_input(|s| ChangePasswordMessage::NewPasswordChanged(s).into())
            .placeholder("New password")
            .password()
            .on_submit(ChangePasswordMessage::Save.into())
            .view();

        let confirm_new_password =
            TextInput::with_label("Confirm password", &self.confirm_new_password)
                .on_input(|s| ChangePasswordMessage::ConfirmNewPasswordChanged(s).into())
                .placeholder("Confirm password")
                .password()
                .on_submit(ChangePasswordMessage::Save.into())
                .view();

        let save_btn = Button::new()
            .text("Save")
            .on_press(ChangePasswordMessage::Save.into())
            .loading(self.loading)
            .width(Length::Fill);

        let content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Change password").big().bold().view())
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(password)
            .push(new_password)
            .push(confirm_new_password)
            .push(if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            })
            .push(save_btn.view())
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<ChangePasswordState> for Box<dyn State> {
    fn from(s: ChangePasswordState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ChangePasswordMessage> for Message {
    fn from(msg: ChangePasswordMessage) -> Self {
        Self::ChangePassword(msg)
    }
}
