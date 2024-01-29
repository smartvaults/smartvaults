// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

include!(concat!(env!("OUT_DIR"), "/signer.rs"));

pub use self::{
    DescriptorKeyValue as ProtoDescriptor, Purpose as ProtoPurpose,
    SharedSigner as ProtoSharedSigner, SharedSignerInvite as ProtoSharedSignerInvite,
    Signer as ProtoSigner, SignerType as ProtoSignerType,
};
