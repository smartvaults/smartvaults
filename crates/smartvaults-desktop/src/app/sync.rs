// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::any::TypeId;
use std::hash::Hash;

use async_stream::stream;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::advanced::Hasher;
use iced::Subscription;
use iced_futures::BoxStream;
use smartvaults_sdk::{Message, SmartVaults};

pub struct SmartVaultsSync {
    client: SmartVaults,
}

impl Recipe for SmartVaultsSync {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<Self::Output> {
        let mut receiver = self.client.sync_notifications();
        let stream = stream! {
            while let Ok(msg) = receiver.recv().await {
                yield msg;
            }
        };
        Box::pin(stream)
    }
}

impl SmartVaultsSync {
    pub fn subscription(client: SmartVaults) -> Subscription<Message> {
        Subscription::from_recipe(Self { client })
    }
}
