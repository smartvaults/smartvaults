// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use super::Wrapper;
use crate::v2::proto::wrapper::{ProtoWrapper, ProtoWrapperObject};
use crate::v2::{Error, SharedSignerInvite, VaultInvite};

impl From<&Wrapper> for ProtoWrapper {
    fn from(wrapper: &Wrapper) -> Self {
        ProtoWrapper {
            object: Some(match wrapper {
                Wrapper::VaultInvite(invite) => ProtoWrapperObject::VaultInvite(invite.into()),
                Wrapper::SharedSignerInvite(invite) => {
                    ProtoWrapperObject::SharedSignerInvite(invite.into())
                }
            }),
        }
    }
}

impl TryFrom<ProtoWrapper> for Wrapper {
    type Error = Error;

    fn try_from(wrapper: ProtoWrapper) -> Result<Self, Self::Error> {
        match wrapper.object {
            Some(obj) => match obj {
                ProtoWrapperObject::VaultInvite(val) => {
                    Ok(Self::VaultInvite(VaultInvite::try_from(val)?))
                }
                ProtoWrapperObject::SharedSignerInvite(val) => {
                    Ok(Self::SharedSignerInvite(SharedSignerInvite::try_from(val)?))
                }
            },
            None => Err(Error::NotFound(String::from("protobuf wrapper obj"))),
        }
    }
}
