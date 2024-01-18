// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

include!(concat!(env!("OUT_DIR"), "/wrapper.rs"));

pub use self::wrapper::Object as ProtoWrapperObject;
pub use self::{
    SharedSignerInvite as ProtoSharedSignerInvite, VaultInvite as ProtoVaultInvite,
    Wrapper as ProtoWrapper,
};
