// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::Coinstr;

use super::screen::{GenerateMessage, OpenMessage, RestoreMessage, SettingMessage};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Open(OpenMessage),
    Restore(RestoreMessage),
    Generate(GenerateMessage),
    Setting(SettingMessage),
    OpenResult(Coinstr),
    Load,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::Start(Box::new(msg))
    }
}
