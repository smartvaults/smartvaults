// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{column, row, svg, Column, PickList, Rule, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::SmartVaults;

use super::view;
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::constants::{APP_DESCRIPTION, APP_LOGO};
use crate::start::{Context, Message, Stage, State};
use crate::theme::color::{DARK_RED, GREY};
use crate::BASE_PATH;

#[derive(Debug, Clone)]
pub enum OpenMessage {
    LoadKeychains,
    KeychainSelect(String),
    PasswordChanged(String),
    ErrorChanged(Option<String>),
    OpenButtonPressed,
}

#[derive(Debug, Default)]
pub struct OpenState {
    keychains: Vec<String>,
    name: Option<String>,
    password: String,
    error: Option<String>,
}

impl OpenState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for OpenState {
    fn title(&self) -> String {
        String::new()
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        Command::perform(async {}, |_| Message::Open(OpenMessage::LoadKeychains))
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Open(msg) = message {
            match msg {
                OpenMessage::LoadKeychains => {
                    match SmartVaults::list_keychains(BASE_PATH.as_path(), ctx.network) {
                        Ok(list) => self.keychains = list,
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                OpenMessage::KeychainSelect(name) => self.name = Some(name),
                OpenMessage::PasswordChanged(psw) => self.password = psw,
                OpenMessage::ErrorChanged(e) => {
                    self.error = e;
                }
                OpenMessage::OpenButtonPressed => {
                    if let Some(name) = self.name.clone() {
                        let network = ctx.network;
                        let password = self.password.clone();
                        return Command::perform(
                            async move {
                                SmartVaults::open(BASE_PATH.as_path(), name, password, network)
                                    .await
                            },
                            move |res| match res {
                                Ok(keechain) => Message::OpenResult(keechain),
                                Err(e) => OpenMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    } else {
                        self.error = Some(String::from("Please, select a keychain"));
                    }
                }
            }
        };

        Command::none()
    }

    fn view(&self, _ctx: &Context) -> Element<Message> {
        let handle = svg::Handle::from_memory(APP_LOGO);
        let svg = svg(handle)
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(120.0));

        let keychain_pick_list = Column::new()
            .push(Text::new("Keychain").view())
            .push(
                PickList::new(self.keychains.clone(), self.name.clone(), |name| {
                    Message::Open(OpenMessage::KeychainSelect(name))
                })
                .width(Length::Fill)
                .padding(10)
                .placeholder(if self.keychains.is_empty() {
                    "No keychain availabe"
                } else {
                    "Select a keychain"
                }),
            )
            .spacing(5);

        let password = TextInput::new("Password", &self.password)
            .on_input(|s| Message::Open(OpenMessage::PasswordChanged(s)))
            .placeholder("Enter password")
            .on_submit(Message::Open(OpenMessage::OpenButtonPressed))
            .password()
            .view();

        let open_btn = Button::new()
            .text("Open")
            .width(Length::Fill)
            .on_press(Message::Open(OpenMessage::OpenButtonPressed))
            .view();

        let new_keychain_btn = Button::new()
            .text("Create keychain")
            .style(ButtonStyle::Bordered)
            .on_press(Message::View(Stage::New))
            .width(Length::Fill)
            .view();

        let restore_keychain_btn = Button::new()
            .text("Restore keychain")
            .style(ButtonStyle::Bordered)
            .on_press(Message::View(Stage::Restore))
            .width(Length::Fill)
            .view();

        let content = column![
            row![column![
                row![svg],
                row![Space::with_height(Length::Fixed(5.0))],
                row![Text::new(APP_DESCRIPTION).big().color(GREY).view()]
            ]
            .align_items(Alignment::Center)
            .spacing(15)],
            row![Space::with_height(Length::Fixed(5.0))],
            row![keychain_pick_list],
            row![password],
            if let Some(error) = &self.error {
                row![Text::new(error).color(DARK_RED).view()]
            } else {
                row![]
            },
            row![open_btn],
            row![Rule::horizontal(1)],
            row![new_keychain_btn],
            row![restore_keychain_btn],
            row![Space::with_height(Length::Fixed(30.0))]
        ];

        view(content)
    }
}

impl From<OpenState> for Box<dyn State> {
    fn from(s: OpenState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<OpenMessage> for Message {
    fn from(msg: OpenMessage) -> Self {
        Self::Open(msg)
    }
}
