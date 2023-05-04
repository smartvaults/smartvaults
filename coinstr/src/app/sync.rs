// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use async_stream::stream;
use coinstr_core::bitcoin::Network;
use coinstr_core::Coinstr;
use iced::Subscription;
use iced_futures::BoxStream;
use tokio::sync::mpsc;

pub struct CoinstrSync {
    client: Coinstr,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl<H, I> iced::subscription::Recipe<H, I> for CoinstrSync
where
    H: std::hash::Hasher,
{
    type Output = ();

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(mut self: Box<Self>, _input: BoxStream<I>) -> BoxStream<Self::Output> {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let endpoint: &str = match self.client.network() {
            Network::Bitcoin => "ssl://blockstream.info:700",
            Network::Testnet => "ssl://blockstream.info:993",
            _ => panic!("Endpoints not availabe for this network"),
        };

        let join = self.client.sync(endpoint, Some(sender));

        self.join = Some(join);
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
        Subscription::from_recipe(Self { client, join: None })
    }
}
