// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::util;
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{ARROW_DOWN, ARROW_UP};

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
}

#[derive(Debug)]
pub struct PolicyState {
    /* loading: bool,
    loaded: bool, */
    policy_id: EventId,
    #[allow(dead_code)]
    policy: Policy,
}

impl PolicyState {
    pub fn new(policy_id: EventId, policy: Policy) -> Self {
        Self {
            /* loading: false,
            loaded: false, */
            policy_id,
            policy,
        }
    }
}

impl State for PolicyState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Policy #{}",
            util::cut_event_id(self.policy_id)
        )
    }

    /* fn load(&mut self, ctx: &Context) -> Command<Message> {
        todo!()
    } */

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        /* if !self.loaded && !self.loading {
            return self.load(ctx);
        } */

        if let Message::Policy(msg) = message {
            match msg {
                PolicyMessage::Send => Command::none(),
                PolicyMessage::Deposit => Command::none(),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        let title = format!("Policy #{}", util::cut_event_id(self.policy_id));
        content = content
            .push(Text::new(title).size(40).bold().view())
            .push(Space::with_height(Length::Fixed(40.0)));

        let send_btn = button::border_text_below_icon(ARROW_UP, "Send")
            .on_press(PolicyMessage::Send.into())
            .width(Length::Fixed(110.0));
        let deposit_btn = button::border_text_below_icon(ARROW_DOWN, "Deposit")
            .on_press(PolicyMessage::Deposit.into())
            .width(Length::Fixed(110.0));
        let row = Row::new()
            .push(
                Row::new()
                    .push(send_btn)
                    .push(deposit_btn)
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(
                Container::new(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("1 050 641").size(45).view())
                                .push(Text::new("50 641").size(27).view())
                                .align_items(Alignment::End),
                        )
                        .push(
                            Column::new()
                                .push(Space::with_height(Length::Fixed(9.5)))
                                .push(Text::new("sats").size(35).view())
                                .push(Space::with_height(Length::Fixed(27.5)))
                                .align_items(Alignment::End),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .align_x(Horizontal::Right),
            )
            .width(Length::Fill)
            .align_items(Alignment::Center);
        content = content.push(row);

        Dashboard::new().view(ctx, content, false, false)
    }
}

impl From<PolicyState> for Box<dyn State> {
    fn from(s: PolicyState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PolicyMessage> for Message {
    fn from(msg: PolicyMessage) -> Self {
        Self::Policy(msg)
    }
}
