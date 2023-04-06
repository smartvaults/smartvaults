// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::Coinstr;

use super::screen::{OpenMessage, RestoreMessage};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Open(OpenMessage),
    Restore(RestoreMessage),
    OpenResult(Coinstr),
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::Start(Box::new(msg))
    }
}
