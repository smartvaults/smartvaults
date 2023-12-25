// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::constants::SMARTVAULTS_ACCOUNT_INDEX;
use smartvaults_sdk::core::hwi::types::HWIDevice;
use smartvaults_sdk::core::hwi::HWIClient;
use smartvaults_sdk::core::signer::Signer;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;
use crate::theme::icon::RELOAD;

#[derive(Debug, Clone)]
pub enum AddHWSignerMessage {
    NameChanged(String),
    SelectDevice(HWIDevice),
    LoadDevices(Vec<HWIDevice>),
    ErrorChanged(Option<String>),
    SaveSigner,
    Reload,
}

#[derive(Debug, Default)]
pub struct AddHWSignerState {
    loading: bool,
    loaded: bool,
    name: String,
    device: Option<HWIDevice>,
    devices: Vec<HWIDevice>,
    error: Option<String>,
}

impl AddHWSignerState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddHWSignerState {
    fn title(&self) -> String {
        String::from("Add signer")
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        Command::perform(
            async move {
                HWIClient::enumerate()
                    .unwrap()
                    .into_iter()
                    .filter_map(|d| d.ok())
                    .collect()
            },
            |devices| AddHWSignerMessage::LoadDevices(devices).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::AddHWSigner(msg) = message {
            match msg {
                AddHWSignerMessage::NameChanged(name) => self.name = name,
                AddHWSignerMessage::SelectDevice(device) => self.device = Some(device),
                AddHWSignerMessage::LoadDevices(devices) => {
                    self.devices = devices;
                    self.loaded = true;
                    self.loading = false;
                }
                AddHWSignerMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                AddHWSignerMessage::SaveSigner => {
                    if let Some(device) = &self.device {
                        self.loading = true;
                        let client = ctx.client.clone();
                        let name = self.name.clone();
                        let device = device.clone();
                        return Command::perform(
                            async move {
                                let signer = Signer::from_hwi(
                                    name,
                                    None,
                                    device,
                                    Some(SMARTVAULTS_ACCOUNT_INDEX),
                                    client.network(),
                                )?;
                                client.save_signer(signer).await?;
                                Ok::<(), Box<dyn std::error::Error>>(())
                            },
                            |res| match res {
                                Ok(_) => Message::View(Stage::Signers),
                                Err(e) => {
                                    AddHWSignerMessage::ErrorChanged(Some(e.to_string())).into()
                                }
                            },
                        );
                    }
                }
                AddHWSignerMessage::Reload => {
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if let Some(device) = &self.device {
                let name = TextInput::with_label("Name", &self.name)
                    .on_input(|s| AddHWSignerMessage::NameChanged(s).into())
                    .placeholder("Name")
                    .view();

                let device_type = TextInput::with_label("Type", &device.device_type.to_string()).view();

                let device_model = TextInput::with_label("Model", &device.model).view();

                let fingerprint = TextInput::with_label("Fingerprint", &device.fingerprint.to_string())
                    .placeholder("Master fingerprint")
                    .view();

                let error = if let Some(error) = &self.error {
                    Row::new().push(Text::new(error).color(DARK_RED).view())
                } else {
                    Row::new()
                };

                content = content
                    .push(
                        Column::new()
                            .push(Text::new("Create signer").big().bold().view())
                            .push(Text::new("Create a new HW signer").extra_light().view())
                            .spacing(10)
                            .width(Length::Fill),
                    )
                    .push(name)
                    .push(device_type)
                    .push(device_model)
                    .push(fingerprint)
                    .push(error)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .text("Save signer")
                            .on_press(AddHWSignerMessage::SaveSigner.into())
                            .loading(self.loading)
                            .width(Length::Fill)
                            .view(),
                    )
                    .align_items(Alignment::Center)
                    .spacing(10)
                    .padding(20)
                    .max_width(400);
            } else if self.devices.is_empty() {
                content = content
                    .push(Text::new("No devices found").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .icon(RELOAD)
                            .text("Reload")
                            .style(ButtonStyle::Bordered)
                            .width(Length::Fixed(250.0))
                            .on_press(AddHWSignerMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                content = content
                    .push(
                        Row::new()
                            .push(Text::new("Type").bold().big().width(Length::Fill).view())
                            .push(Text::new("Model").bold().big().width(Length::Fill).view())
                            .push(
                                Text::new("Fingerprint")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .push(
                                Button::new()
                                    .icon(RELOAD)
                                    .style(ButtonStyle::Bordered)
                                    .width(Length::Fixed(40.0))
                                    .on_press(AddHWSignerMessage::Reload.into())
                                    .loading(self.loading)
                                    .view(),
                            )
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for device in self.devices.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(device.device_type.to_string())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(device.model.to_string())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(device.fingerprint.to_string())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Button::new()
                                .text("Select")
                                .on_press(AddHWSignerMessage::SelectDevice(device.clone()).into())
                                .width(Length::Fixed(90.0))
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

impl From<AddHWSignerState> for Box<dyn State> {
    fn from(s: AddHWSignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddHWSignerMessage> for Message {
    fn from(msg: AddHWSignerMessage) -> Self {
        Self::AddHWSigner(msg)
    }
}
