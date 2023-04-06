// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

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
}

impl Context {
    pub fn new(stage: Stage) -> Self {
        Self { stage }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
