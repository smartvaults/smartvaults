// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::button;

#[derive(Debug, Clone)]
pub enum AddSignerMessage {
    LoadCoinstrSigner(bool),
    AddCoinstrSigner,
}

#[derive(Debug, Default)]
pub struct AddSignerState {
    loading: bool,
    loaded: bool,
    coinstr_signer_exists: bool,
}

impl AddSignerState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddSignerState {
    fn title(&self) -> String {
        String::from("Add signer")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.coinstr_signer_exists().unwrap() },
            |value| AddSignerMessage::LoadCoinstrSigner(value).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddSigner(msg) = message {
            match msg {
                AddSignerMessage::LoadCoinstrSigner(value) => {
                    self.coinstr_signer_exists = value;
                    self.loading = false;
                    self.loaded = true;
                }
                AddSignerMessage::AddCoinstrSigner => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.save_coinstr_signer().await.unwrap() },
                        |_| Message::View(Stage::Signers),
                    );
                }
            }
        }
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut add_coinstr_signer_btn = button::primary("Coinstr Signer").width(Length::Fill);

        if self.loaded && !self.coinstr_signer_exists {
            add_coinstr_signer_btn =
                add_coinstr_signer_btn.on_press(AddSignerMessage::AddCoinstrSigner.into());
        }

        let content = Column::new()
            .push(add_coinstr_signer_btn)
            .push(
                button::primary("Connect Signing Device")
                    .on_press(Message::View(Stage::AddHWSigner))
                    .width(Length::Fill),
            )
            .push(
                button::primary("Add AirGap Signer")
                    .on_press(Message::View(Stage::AddAirGapSigner))
                    .width(Length::Fill),
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<AddSignerState> for Box<dyn State> {
    fn from(s: AddSignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddSignerMessage> for Message {
    fn from(msg: AddSignerMessage) -> Self {
        Self::AddSigner(msg)
    }
}
