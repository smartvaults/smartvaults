// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::ops::Sub;
use std::time::Duration;

use async_recursion::async_recursion;
use async_stream::stream;
use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bitcoin::Network;
use coinstr_core::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, POLICY_KIND,
    SPENDING_PROPOSAL_KIND,
};
use coinstr_core::nostr_sdk::prelude::TagKind;
use coinstr_core::nostr_sdk::{
    Event, EventId, Filter, Keys, RelayPoolNotification, Result, Tag, Timestamp,
};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::{ApprovedProposal, SpendingProposal};
use coinstr_core::{util, CoinstrClient, Encryption};
use futures_util::future::{AbortHandle, Abortable};
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
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let bitcoin_endpoint: &str = match self.client.network() {
            Network::Bitcoin => "ssl://blockstream.info:700",
            Network::Testnet => "ssl://blockstream.info:993",
            _ => panic!("Endpoints not availabe for this network"),
        };

        let client = self.client.clone();
        let cache = self.cache.clone();
        let join = tokio::task::spawn(async move {
            // Sync wallet thread
            let ccache = cache.clone();
            let ssender = sender.clone();
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let wallet_sync = async move {
                let electrum_client = ElectrumClient::new(bitcoin_endpoint).unwrap();
                let blockchain = ElectrumBlockchain::from(electrum_client);
                loop {
                    if let Err(e) = ccache
                        .sync_wallets(&blockchain, Some(&ssender), false)
                        .await
                    {
                        log::error!("Impossible to sync wallets: {e}");
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            };
            let future = Abortable::new(wallet_sync, abort_registration);
            tokio::task::spawn(async {
                let _ = future.await;
                log::debug!("Exited from wallet sync thread");
            });

            let nostr_client = client.inner();
            let keys = nostr_client.keys();

            let mut shared_keys = client
                .get_shared_keys(Some(Duration::from_secs(60)))
                .await
                .unwrap_or_default();

            log::info!("Got shared keys");

            let filters = vec![
                Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(SPENDING_PROPOSAL_KIND),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(APPROVED_PROPOSAL_KIND)
                    .since(Timestamp::now().sub(APPROVED_PROPOSAL_EXPIRATION)),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(COMPLETED_PROPOSAL_KIND),
            ];

            nostr_client.subscribe(filters).await;

            let mut notifications = nostr_client.notifications();
            while let Ok(notification) = notifications.recv().await {
                match notification {
                    RelayPoolNotification::Event(_, event) => {
                        let event_id = event.id;
                        match handle_event(&client, &cache, &mut shared_keys, event).await {
                            Ok(_) => {
                                sender.send(()).ok();
                            }
                            Err(e) => log::error!("Impossible to handle event {event_id}: {e}"),
                        }
                    }
                    RelayPoolNotification::Shutdown => {
                        abort_handle.abort();
                        break;
                    }
                    _ => (),
                }
            }
            log::debug!("Exited from nostr sync thread");
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
    if event.kind == POLICY_KIND && !cache.policy_exists(event.id).await {
        if let Some(shared_key) = shared_keys.get(&event.id) {
            let policy = Policy::decrypt(shared_key, &event.content)?;
            cache
                .cache_policy(event.id, policy, client.network())
                .await?;
        } else {
            log::info!("Requesting shared key for {}", event.id);
            tokio::time::sleep(Duration::from_secs(1)).await;
            let shared_key = client
                .get_shared_key_by_policy_id(event.id, Some(Duration::from_secs(30)))
                .await?;
            shared_keys.insert(event.id, shared_key);
            handle_event(client, cache, shared_keys, event).await?;
        }
    } else if event.kind == SPENDING_PROPOSAL_KIND && !cache.proposal_exists(event.id).await {
        if let Some(policy_id) = util::extract_first_event_id(&event) {
            if let Some(shared_key) = shared_keys.get(&policy_id) {
                let proposal = SpendingProposal::decrypt(shared_key, &event.content)?;
                cache.cache_proposal(event.id, policy_id, proposal).await;
            } else {
                log::info!("Requesting shared key for proposal {}", event.id);
                tokio::time::sleep(Duration::from_secs(1)).await;
                let shared_key = client
                    .get_shared_key_by_policy_id(policy_id, Some(Duration::from_secs(30)))
                    .await?;
                shared_keys.insert(policy_id, shared_key);
                handle_event(client, cache, shared_keys, event).await?;
            }
        } else {
            log::error!("Impossible to find policy id in proposal {}", event.id);
        }
    } else if event.kind == APPROVED_PROPOSAL_KIND {
        if let Some(proposal_id) = util::extract_first_event_id(&event) {
            if let Some(Tag::Event(policy_id, ..)) =
                util::extract_tags_by_kind(&event, TagKind::E).get(1)
            {
                if let Some(shared_key) = shared_keys.get(policy_id) {
                    let approved_proposal = ApprovedProposal::decrypt(shared_key, &event.content)?;
                    cache
                        .cache_approved_proposal(
                            proposal_id,
                            event.pubkey,
                            event.id,
                            approved_proposal.psbt(),
                            event.created_at,
                        )
                        .await;
                } else {
                    log::info!("Requesting shared key for approved proposal {}", event.id);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let shared_key = client
                        .get_shared_key_by_policy_id(*policy_id, Some(Duration::from_secs(30)))
                        .await?;
                    shared_keys.insert(*policy_id, shared_key);
                    handle_event(client, cache, shared_keys, event).await?;
                }
            } else {
                log::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else {
            log::error!(
                "Impossible to find proposal id in approved proposal {}",
                event.id
            );
        }
    } else if event.kind == COMPLETED_PROPOSAL_KIND {
        if let Some(proposal_id) = util::extract_first_event_id(&event) {
            cache.uncache_proposal(proposal_id).await;
        }
    }

    Ok(())
}
