// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use nostr_sdk::nips::nip46::NostrConnectURI;
use nostr_sdk::nips::nip46::{Message as NIP46Message, Request as NIP46Request};
use nostr_sdk::{ClientMessage, EventBuilder, EventId, Keys, Timestamp, Url};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk_sqlite::model::NostrConnectRequest;

use super::{Error, SmartVaults};

impl SmartVaults {
    pub async fn new_nostr_connect_session(&self, uri: NostrConnectURI) -> Result<(), Error> {
        let relay_url: Url = uri.relay_url.clone();

        // Try to add relay and check if it's already added
        if self.client.add_relay(&relay_url).await? {
            let relay = self.client.relay(&relay_url).await?;
            relay.connect(true).await;

            let last_sync: Timestamp = match self.db.get_last_relay_sync(relay_url.clone()).await {
                Ok(ts) => ts,
                Err(e) => {
                    tracing::error!("Impossible to get last relay sync: {e}");
                    Timestamp::from(0)
                }
            };
            let filters = self.sync_filters(last_sync).await;
            relay.subscribe(filters, None).await?;
        }

        // Send connect ACK
        let keys: Keys = self.keys().await;
        let msg = NIP46Message::request(NIP46Request::Connect(keys.public_key()));
        let nip46_event =
            EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
        self.client.send_event_to(relay_url, nip46_event).await?;

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
        app_public_key: XOnlyPublicKey,
        wait: bool,
    ) -> Result<(), Error> {
        let uri = self.db.get_nostr_connect_session(app_public_key).await?;
        let keys: Keys = self.keys().await;
        let msg = NIP46Message::request(NIP46Request::Disconnect);
        let nip46_event =
            EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
        if wait {
            self.client
                .send_event_to(uri.relay_url, nip46_event)
                .await?;
        } else {
            self.client
                .pool()
                .send_msg_to(uri.relay_url, ClientMessage::new_event(nip46_event), None)
                .await?;
        }
        self.db.delete_nostr_connect_session(app_public_key).await?;
        Ok(())
    }

    pub async fn disconnect_nostr_connect_session(
        &self,
        app_public_key: XOnlyPublicKey,
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
            let keys: Keys = self.keys().await;
            let msg = message
                .generate_response(&keys)?
                .ok_or(Error::CantGenerateNostrConnectResponse)?;
            let nip46_event =
                EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
            self.client
                .send_event_to(uri.relay_url, nip46_event)
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
            let keys: Keys = self.keys().await;
            let msg = message.generate_error_response("Request rejected")?; // TODO: better error msg
            let nip46_event =
                EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
            self.client
                .send_event_to(uri.relay_url, nip46_event)
                .await?;
            self.db.delete_nostr_connect_request(event_id).await?;
            Ok(())
        } else {
            Err(Error::NostrConnectRequestAlreadyApproved)
        }
    }

    pub async fn auto_approve_nostr_connect_requests(
        &self,
        app_public_key: XOnlyPublicKey,
        duration: Duration,
    ) {
        let until: Timestamp = Timestamp::now() + duration;
        self.db
            .set_nostr_connect_auto_approve(app_public_key, until)
            .await;
    }

    pub async fn revoke_nostr_connect_auto_approve(&self, app_public_key: XOnlyPublicKey) {
        self.db
            .revoke_nostr_connect_auto_approve(app_public_key)
            .await;
    }

    pub async fn get_nostr_connect_pre_authorizations(
        &self,
    ) -> BTreeMap<XOnlyPublicKey, Timestamp> {
        self.db.get_nostr_connect_pre_authorizations().await
    }
}
