// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use super::screen::{PoliciesMessage, PolicyMessage};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Policies(PoliciesMessage),
    Policy(PolicyMessage),
    Lock,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}