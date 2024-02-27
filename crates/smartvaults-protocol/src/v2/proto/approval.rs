// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

include!(concat!(env!("OUT_DIR"), "/approval.rs"));

pub use self::approval::Object as ProtoApprovalObject;
pub use self::{
    Approval as ProtoApproval, ApprovalType as ProtoApprovalType, ApprovalV1 as ProtoApprovalV1,
};
