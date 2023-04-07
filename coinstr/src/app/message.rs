// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use super::screen::PoliciesMessage;
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    PoliciesMessage(PoliciesMessage),
    Lock,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
