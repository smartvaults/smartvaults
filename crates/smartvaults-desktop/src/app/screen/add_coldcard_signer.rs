// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;
use smartvaults_sdk::core::ColdcardGenericJson;
use smartvaults_sdk::prelude::bips::bip48::ScriptType;
use smartvaults_sdk::prelude::Purpose;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;

const PURPOSE: Purpose = Purpose::BIP48 {
    script: ScriptType::P2TR,
};

#[derive(Debug, Clone)]
pub enum AddColdcardSignerMessage {
    NameChanged(String),
    SelectGenericJson,
    LoadGenericJson(ColdcardGenericJson),
    ReadOnly,
    ErrorChanged(Option<String>),
    SaveSigner,
}

#[derive(Debug, Default)]
pub struct AddColdcardSignerState {
    name: String,
    generic_json: Option<ColdcardGenericJson>,
    loading: bool,
    error: Option<String>,
}

impl AddColdcardSignerState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddColdcardSignerState {
    fn title(&self) -> String {
        String::from("Add signer")
    }

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddColdcardSigner(msg) = message {
            match msg {
                AddColdcardSignerMessage::NameChanged(name) => self.name = name,
                AddColdcardSignerMessage::SelectGenericJson => {
                    let path = FileDialog::new()
                        .set_title("Select Coldcard generic JSON")
                        .pick_file();

                    if let Some(path) = path {
                        return Command::perform(
                            async move { ColdcardGenericJson::from_file(path) },
                            |res| match res {
                                Ok(generic_json) => {
                                    AddColdcardSignerMessage::LoadGenericJson(generic_json).into()
                                }
                                Err(e) => {
                                    AddColdcardSignerMessage::ErrorChanged(Some(e.to_string()))
                                        .into()
                                }
                            },
                        );
                    }
                }
                AddColdcardSignerMessage::LoadGenericJson(generic_json) => {
                    self.generic_json = Some(generic_json)
                }
                AddColdcardSignerMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                AddColdcardSignerMessage::SaveSigner => match &self.generic_json {
                    Some(_generic_json) => {
                        todo!();

                        // self.loading = true;
                        // let client = ctx.client.clone();
                        // let name = self.name.clone();
                        // let coldcard = generic_json.clone();
                        // return Command::perform(
                        // async move {
                        // let signer =
                        // Signer::from_coldcard(name, coldcard, client.network())?;
                        // client.save_signer(signer).await?;
                        // Ok::<(), Box<dyn std::error::Error>>(())
                        // },
                        // |res| match res {
                        // Ok(_) => Message::View(Stage::Signers),
                        // Err(e) => {
                        // AddColdcardSignerMessage::ErrorChanged(Some(e.to_string()))
                        // .into()
                        // }
                        // },
                        // );
                    }
                    None => {
                        self.error = Some(String::from("No Coldcard generic JSON selected."));
                    }
                },
                AddColdcardSignerMessage::ReadOnly => (),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Create Coldcard signer").big().bold().view())
                    .push(
                        Text::new("In your Coldcard go to: Advanced/Tools -> Export Wallet -> Generic JSON")
                            .extra_light()
                            .view(),
                    )
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(TextInput::with_label("Name", &self.name)
            .on_input(|s| AddColdcardSignerMessage::NameChanged(s).into())
            .placeholder("Name")
            .view());

        if let Some(generic_json) = &self.generic_json {
            let desc: String = generic_json
                .descriptor(PURPOSE)
                .map_or(String::from("Descriptor not found"), |d| d.to_string());
            let fingerprint =
                TextInput::with_label("Fingerprint", &generic_json.fingerprint().to_string())
                    .on_input(|_| AddColdcardSignerMessage::ReadOnly.into())
                    .view();
            let descriptor = TextInput::with_label("Descriptor", &desc)
                .on_input(|_| AddColdcardSignerMessage::ReadOnly.into())
                .view();
            content = content.push(fingerprint).push(descriptor);
        }

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        content = content
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(
                Button::new()
                    .text("Select generic JSON")
                    .style(ButtonStyle::Bordered)
                    .on_press(AddColdcardSignerMessage::SelectGenericJson.into())
                    .loading(self.loading)
                    .width(Length::Fill)
                    .view(),
            )
            .push(
                Button::new()
                    .text("Save signer")
                    .on_press(AddColdcardSignerMessage::SaveSigner.into())
                    .loading(self.loading)
                    .width(Length::Fill)
                    .view(),
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<AddColdcardSignerState> for Box<dyn State> {
    fn from(s: AddColdcardSignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddColdcardSignerMessage> for Message {
    fn from(msg: AddColdcardSignerMessage) -> Self {
        Self::AddColdcardSigner(msg)
    }
}
