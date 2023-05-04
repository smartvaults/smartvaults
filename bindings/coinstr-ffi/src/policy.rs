// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::ops::Deref;

use coinstr_core::policy;

#[derive(Clone)]
pub struct Policy {
    inner: policy::Policy,
}

impl Deref for Policy {
    type Target = policy::Policy;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Policy {
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    pub fn descriptor(&self) -> String {
        self.inner.descriptor.to_string()
    }
}
