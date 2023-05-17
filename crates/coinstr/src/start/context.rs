// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::Network;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Open,
    New,
    Restore,
}

impl Default for Stage {
    fn default() -> Self {
        Self::Open
    }
}

pub struct Context {
    pub stage: Stage,
    pub network: Network,
    pub theme: Theme,
}

impl Context {
    pub fn new(stage: Stage, network: Network, theme: Theme) -> Self {
        Self {
            stage,
            network,
            theme,
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
