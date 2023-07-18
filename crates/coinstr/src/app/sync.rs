// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::any::TypeId;
use std::hash::{Hash, Hasher};

use async_stream::stream;
use coinstr_sdk::client::Message;
use coinstr_sdk::Coinstr;
use iced::Subscription;
use iced_futures::BoxStream;
use notify_rust::Notification as DesktopNotification;

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
        let mut receiver = self.client.sync_notifications();
        let stream = stream! {
            while let Ok(item) = receiver.recv().await {
                if let Some(Message::Notification(notification)) = item {
                    if let Err(e) = DesktopNotification::new().summary("Coinstr").body(&notification.to_string()).show() {
                        log::error!("Impossible to send desktop notification: {e}");
                    };
                }
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
