// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::signer::Signer;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::color::RED;
use crate::theme::icon::TRASH;

#[derive(Debug, Clone)]
pub enum SignerMessage {
    Delete,
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct SignerState {
    loading: bool,
    loaded: bool,
    signer_id: EventId,
    signer: Signer,
    error: Option<String>,
}

impl SignerState {
    pub fn new(signer_id: EventId, signer: Signer) -> Self {
        Self {
            loading: false,
            loaded: false,
            signer_id,
            signer,
            error: None,
        }
    }
}

impl State for SignerState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Signer #{}",
            util::cut_event_id(self.signer_id)
        )
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        /* let client = ctx.client.clone();
        let signer_id = self.signer_id; */
        self.loading = false;
        self.loaded = true;
        Command::none()
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Signer(msg) = message {
            match msg {
                SignerMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SignerMessage::Delete => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let signer_id = self.signer_id;
                    return Command::perform(
                        async move { client.delete_signer_by_id(signer_id, None).await },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Signers),
                            Err(e) => SignerMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
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
                    Text::new(format!("Signer #{}", util::cut_event_id(self.signer_id)))
                        .size(40)
                        .bold()
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(Text::new(format!("Name: {}", self.signer.name())).view())
                .push(Text::new(format!("Type: {}", self.signer.signer_type())).view())
                .push(Text::new(format!("Fingerprint: {}", self.signer.fingerprint())).view())
                .push(Text::new(format!("Descriptor: {}", self.signer.descriptor())).view());

            let mut delete_btn = button::danger_with_icon(TRASH, "Delete");

            if !self.loading {
                delete_btn = delete_btn.on_press(SignerMessage::Delete.into());
            }

            content = content
                .push(Space::with_height(10.0))
                .push(Row::new().push(delete_btn).spacing(10))
                .push(Space::with_height(20.0));

            if let Some(error) = &self.error {
                content = content.push(Text::new(error).color(RED).view());
            };

            // TODO: show shared signers (nostr pubkey, timestamp and revoke btn)
        } else {
            content = content.push(Text::new("Loading...").view())
        };

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<SignerState> for Box<dyn State> {
    fn from(s: SignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SignerMessage> for Message {
    fn from(msg: SignerMessage) -> Self {
        Self::Signer(msg)
    }
}