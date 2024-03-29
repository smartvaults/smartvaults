// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::Button;

#[derive(Debug, Clone)]
pub enum AddSignerMessage {
    LoadSmartVaultsSigner(bool),
    AddSmartVaultsSigner,
}

#[derive(Debug, Default)]
pub struct AddSignerState {
    loading: bool,
    loaded: bool,
    smartvaults_signer_exists: bool,
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
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.smartvaults_signer_exists().await },
            |value| AddSignerMessage::LoadSmartVaultsSigner(value).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddSigner(msg) = message {
            match msg {
                AddSignerMessage::LoadSmartVaultsSigner(value) => {
                    self.smartvaults_signer_exists = value;
                    self.loading = false;
                    self.loaded = true;
                }
                AddSignerMessage::AddSmartVaultsSigner => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.save_smartvaults_signer().await.unwrap() },
                        |_| Message::View(Stage::Signers),
                    );
                }
            }
        }
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        #[allow(unused_mut)]
        let mut content = Column::new()
            .push(
                Button::new()
                    .text("Smart Vaults Signer")
                    .on_press(AddSignerMessage::AddSmartVaultsSigner.into())
                    .loading(!self.loaded || self.smartvaults_signer_exists)
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Add Coldcard Signer")
                    .on_press(Message::View(Stage::AddColdcardSigner))
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Add AirGap Signer")
                    .on_press(Message::View(Stage::AddAirGapSigner))
                    .width(Length::Fill)
                    .view(),
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        // {
        // content = content.push(
        // Button::new()
        // .text("Connect Signing Device")
        // .on_press(Message::View(Stage::AddHWSigner))
        // .width(Length::Fill)
        // .view(),
        // );
        // }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
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
