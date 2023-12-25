// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::SmartVaults;

use super::screen::{GenerateMessage, OpenMessage, RestoreMessage, SettingMessage};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Open(OpenMessage),
    Restore(RestoreMessage),
    Generate(GenerateMessage),
    Setting(SettingMessage),
    OpenResult(SmartVaults),
    Load,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::Start(Box::new(msg))
    }
}
