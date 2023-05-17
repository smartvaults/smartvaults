// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::any::TypeId;
use std::hash::{Hash, Hasher};

use async_stream::stream;
use coinstr_sdk::Coinstr;
use iced::Subscription;
use iced_futures::BoxStream;

pub struct CoinstrSync {
    client: Coinstr,
}

impl<H, I> iced::subscription::Recipe<H, I> for CoinstrSync
where
    H: Hasher,
{
    type Output = ();

    fn hash(&self, state: &mut H) {
        TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<I>) -> BoxStream<Self::Output> {
        let mut receiver = self.client.sync();
        let stream = stream! {
            while let Some(item) = receiver.recv().await {
                yield item;
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
