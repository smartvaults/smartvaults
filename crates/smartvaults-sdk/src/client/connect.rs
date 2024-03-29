// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use nostr_sdk::nips::nip46::{Message as NIP46Message, NostrConnectURI, Request as NIP46Request};
use nostr_sdk::{
    ClientMessage, EventBuilder, EventId, Keys, PublicKey, RelaySendOptions, SubscribeOptions,
    SubscriptionId, Timestamp, Url,
};
use smartvaults_sdk_sqlite::model::NostrConnectRequest;

use super::{Error, SmartVaults};
use crate::constants::NOSTR_CONNECT_SUBSCRIPTION_ID;

impl SmartVaults {
    pub async fn new_nostr_connect_session(&self, uri: NostrConnectURI) -> Result<(), Error> {
        let relay_url: Url = uri.relay_url.clone();

        // Try to add relay and check if it's already added
        if self.client.add_relay(&relay_url).await? {
            let relay = self.client.relay(&relay_url).await?;
            relay.connect(Some(Duration::from_secs(30))).await;

            let last_sync: Timestamp = match self.db.get_last_relay_sync(relay_url.clone()).await {
                Ok(ts) => ts,
                Err(e) => {
                    tracing::error!("Impossible to get last relay sync: {e}");
                    Timestamp::from(0)
                }
            };
            let filters = self.sync_filters(last_sync).await;
            relay
                .subscribe_with_id(
                    SubscriptionId::new(NOSTR_CONNECT_SUBSCRIPTION_ID),
                    filters,
                    SubscribeOptions::default(),
                )
                .await?;
        }

        // Send connect ACK
        let keys: &Keys = self.keys();
        let msg = NIP46Message::request(NIP46Request::Connect(keys.public_key()));
        let nip46_event = EventBuilder::nostr_connect(keys, uri.public_key, msg)?.to_event(keys)?;
        self.client.send_event_to([relay_url], nip46_event).await?;

        self.db.save_nostr_connect_uri(uri).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_nostr_connect_sessions(
        &self,
    ) -> Result<Vec<(NostrConnectURI, Timestamp)>, Error> {
        Ok(self.db.get_nostr_connect_sessions().await?)
    }

    pub(crate) async fn _disconnect_nostr_connect_session(
        &self,
        app_public_key: PublicKey,
        wait: bool,
    ) -> Result<(), Error> {
        let uri = self.db.get_nostr_connect_session(app_public_key).await?;
        let keys: &Keys = self.keys();
        let msg = NIP46Message::request(NIP46Request::Disconnect);
        let nip46_event = EventBuilder::nostr_connect(keys, uri.public_key, msg)?.to_event(keys)?;
        if wait {
            self.client
                .send_event_to([uri.relay_url], nip46_event)
                .await?;
        } else {
            self.client
                .pool()
                .send_msg_to(
                    [uri.relay_url],
                    ClientMessage::event(nip46_event),
                    RelaySendOptions::new().skip_send_confirmation(true),
                )
                .await?;
        }
        self.db.delete_nostr_connect_session(app_public_key).await?;
        Ok(())
    }

    pub async fn disconnect_nostr_connect_session(
        &self,
        app_public_key: PublicKey,
    ) -> Result<(), Error> {
        self._disconnect_nostr_connect_session(app_public_key, true)
            .await
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_nostr_connect_requests(
        &self,
        approved: bool,
    ) -> Result<Vec<NostrConnectRequest>, Error> {
        Ok(self.db.get_nostr_connect_requests(approved).await?)
    }

    pub async fn approve_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        let NostrConnectRequest {
            app_public_key,
            message,
            approved,
            ..
        } = self.db.get_nostr_connect_request(event_id).await?;
        if !approved {
            let uri = self.db.get_nostr_connect_session(app_public_key).await?;
            let keys: &Keys = self.keys();
            let msg = message
                .generate_response(keys)?
                .ok_or(Error::CantGenerateNostrConnectResponse)?;
            let nip46_event =
                EventBuilder::nostr_connect(keys, uri.public_key, msg)?.to_event(keys)?;
            self.client
                .send_event_to([uri.relay_url], nip46_event)
                .await?;
            self.db
                .set_nostr_connect_request_as_approved(event_id)
                .await?;
            Ok(())
        } else {
            Err(Error::NostrConnectRequestAlreadyApproved)
        }
    }

    pub async fn reject_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        let NostrConnectRequest {
            app_public_key,
            message,
            approved,
            ..
        } = self.db.get_nostr_connect_request(event_id).await?;
        if !approved {
            let uri = self.db.get_nostr_connect_session(app_public_key).await?;
            let keys: &Keys = self.keys();
            let msg = message.generate_error_response("Request rejected")?; // TODO: better error msg
            let nip46_event =
                EventBuilder::nostr_connect(keys, uri.public_key, msg)?.to_event(keys)?;
            self.client
                .send_event_to([uri.relay_url], nip46_event)
                .await?;
            self.db.delete_nostr_connect_request(event_id).await?;
            Ok(())
        } else {
            Err(Error::NostrConnectRequestAlreadyApproved)
        }
    }

    pub async fn auto_approve_nostr_connect_requests(
        &self,
        app_public_key: PublicKey,
        duration: Duration,
    ) {
        let until: Timestamp = Timestamp::now() + duration;
        self.db
            .set_nostr_connect_auto_approve(app_public_key, until)
            .await;
    }

    pub async fn revoke_nostr_connect_auto_approve(&self, app_public_key: PublicKey) {
        self.db
            .revoke_nostr_connect_auto_approve(app_public_key)
            .await;
    }

    pub async fn get_nostr_connect_pre_authorizations(&self) -> BTreeMap<PublicKey, Timestamp> {
        self.db.get_nostr_connect_pre_authorizations().await
    }
}
