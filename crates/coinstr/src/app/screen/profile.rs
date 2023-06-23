// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::util;
use iced::widget::Column;
use iced::{Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::CLIPBOARD;

#[derive(Debug, Clone)]
pub enum ProfileMessage {}

#[derive(Debug, Default)]
pub struct ProfileState {
    loading: bool,
    loaded: bool,
}

impl ProfileState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ProfileState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Profile")
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        self.loaded = true;
        Command::none()
    }

    fn update(&mut self, ctx: &mut Context, _message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
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

            let public_key = ctx.client.keys().public_key();

            content = content
                .push(Text::new(util::cut_public_key(public_key)).view())
                .push(
                    button::border_only_icon(CLIPBOARD)
                        .on_press(Message::Clipboard(public_key.to_string()))
                        .width(Length::Fixed(40.0)),
                );
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<ProfileState> for Box<dyn State> {
    fn from(s: ProfileState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProfileMessage> for Message {
    fn from(msg: ProfileMessage) -> Self {
        Self::Profile(msg)
    }
}
