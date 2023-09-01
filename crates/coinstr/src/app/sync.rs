// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::any::TypeId;
use std::hash::Hash;

use async_stream::stream;
use coinstr_sdk::Coinstr;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::advanced::Hasher;
use iced::Subscription;
use iced_futures::BoxStream;

pub struct CoinstrSync {
    client: Coinstr,
}

impl Recipe for CoinstrSync {
    type Output = ();

    fn hash(&self, state: &mut Hasher) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<Self::Output> {
        let mut receiver = self.client.sync_notifications();
        let stream = stream! {
            while let Ok(_msg) = receiver.recv().await {
                // TODO
                yield ();
            }
        };
        Box::pin(stream)
    }
}

impl CoinstrSync {
    pub fn subscription(client: Coinstr) -> Subscription<()> {
        Subscription::from_recipe(Self { client })
    }
}
