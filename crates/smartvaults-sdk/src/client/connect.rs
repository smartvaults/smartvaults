// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use nostr_sdk::nips::nip46::{Message as NIP46Message, NostrConnectURI, Request as NIP46Request};
use nostr_sdk::{
    ClientMessage, EventBuilder, EventId, Filter, Keys, Kind, PublicKey, RelaySendOptions, SubscribeOptions, SubscriptionId, Timestamp, Url
};
use smartvaults_sdk_sqlite::model::NostrConnectRequest;

use super::{Error, SmartVaults};
use crate::constants::NOSTR_CONNECT_SUBSCRIPTION_ID;

impl SmartVaults {
    pub async fn new_nostr_connect_session(&self, uri: NostrConnectURI) -> Result<(), Error> {
        if let NostrConnectURI::Client { public_key, relays, .. } = &uri {
            let keys: &Keys = self.keys();

            // Add relays
            self.client.add_relays(relays).await?;
            self.client.connect().await;

            // Subscribe
            let filters = vec![Filter::new()
            .pubkey(keys.public_key())
            .kind(Kind::NostrConnect)
            .since(Timestamp::now())];
            for url in relays.iter() {
                let relay = self.client.relay(url).await?;
                relay
                .subscribe_with_id(
                    SubscriptionId::new(NOSTR_CONNECT_SUBSCRIPTION_ID),
                    filters.clone(),
                    SubscribeOptions::default(),
                )
                .await?;
            }

            // Send connect ACK
            let msg = NIP46Message::request(NIP46Request::Connect{public_key: keys.public_key(), secret: None});
            let nip46_event = EventBuilder::nostr_connect(keys, *public_key, msg)?.to_event(keys)?;
            self.client.send_event_to(relays, nip46_event).await?;

            self.db.save_nostr_connect_uri(uri).await?;

            Ok(())
        } else {
            Err(Error::UnexpectedNostrConnectUri)
        }
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
        let msg = NIP46Message::request(NIP46Request::);
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
