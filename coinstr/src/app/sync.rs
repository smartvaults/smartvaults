// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::collections::HashMap;
use std::time::Duration;

use async_recursion::async_recursion;
use async_stream::stream;
use coinstr_core::constants::POLICY_KIND;
use coinstr_core::nostr_sdk::{nips, Event, EventId, Filter, Keys, RelayPoolNotification, Result};
use coinstr_core::policy::Policy;
use coinstr_core::CoinstrClient;
use iced::Subscription;
use iced_futures::BoxStream;
use tokio::sync::mpsc;

use super::cache::Cache;

pub struct CoinstrSync {
    client: CoinstrClient,
    cache: Cache,
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
        let (_sender, mut receiver) = mpsc::unbounded_channel();

        let client = self.client.clone();
        let cache = self.cache.clone();
        let join = tokio::task::spawn(async move {
            let nostr_client = client.inner();
            let keys = nostr_client.keys();

            let mut shared_keys = client
                .get_shared_keys(Some(Duration::from_secs(60)))
                .await
                .unwrap_or_default();

            log::info!("Got shared keys");

            let filters = vec![Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND)];

            nostr_client.subscribe(filters).await;

            let mut notifications = nostr_client.notifications();
            while let Ok(notification) = notifications.recv().await {
                match notification {
                    RelayPoolNotification::Event(_, event) => {
                        let event_id = event.id;
                        if let Err(e) = handle_event(&client, &cache, &mut shared_keys, event).await
                        {
                            log::error!("Impossible to handle event {event_id}: {e}");
                        }
                        //sender.send(()).ok();
                    }
                    RelayPoolNotification::Shutdown => break,
                    _ => (),
                }
            }
            log::debug!("Exited from sync thread");
        });

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
    pub fn subscription(client: CoinstrClient, cache: Cache) -> Subscription<()> {
        Subscription::from_recipe(Self {
            client,
            cache,
            join: None,
        })
    }
}

#[async_recursion]
async fn handle_event(
    client: &CoinstrClient,
    cache: &Cache,
    shared_keys: &mut HashMap<EventId, Keys>,
    event: Event,
) -> Result<()> {
    if event.kind == POLICY_KIND && !cache.policy_exists(event.id)? {
        if let Some(shared_key) = shared_keys.get(&event.id) {
            let content = nips::nip04::decrypt(
                &shared_key.secret_key()?,
                &shared_key.public_key(),
                &event.content,
            )?;
            let policy = Policy::from_json(content)?;
            cache.insert_policy(event.id, policy)?;
        } else {
            log::info!("Requesting shared key for {}", event.id);
            tokio::time::sleep(Duration::from_secs(5)).await;
            let shared_key = client
                .get_shared_key_by_policy_id(event.id, Some(Duration::from_secs(30)))
                .await?;
            shared_keys.insert(event.id, shared_key);
            handle_event(client, cache, shared_keys, event).await?;
        }
    }

    Ok(())
}