// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

pub use keechain_core::bitcoin::absolute::LockTime as AbsoluteLockTime;
pub use keechain_core::bitcoin::Sequence;
use keechain_core::miniscript::policy::concrete::Policy;
use keechain_core::miniscript::DescriptorPublicKey;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid threshold")]
    InvalidThreshold,
    #[error("not keys")]
    NoKeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum Locktime {
    /// An absolute locktime restriction
    After(AbsoluteLockTime),
    /// A relative locktime restriction
    Older(Sequence),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum DecayingTime {
    Single(Locktime),
    Multiple(Vec<Locktime>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum PolicyTemplateType {
    Multisig,
    /// Social Recovery / Inheritance
    Recovery,
    Hold,
    Decaying,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct RecoveryTemplate {
    threshold: usize,
    keys: Vec<DescriptorPublicKey>,
    timelock: Locktime,
}

impl RecoveryTemplate {
    pub fn new(threshold: usize, keys: Vec<DescriptorPublicKey>, timelock: Locktime) -> Self {
        Self {
            threshold,
            keys,
            timelock,
        }
    }

    pub(crate) fn build(self) -> Result<Policy<DescriptorPublicKey>, Error> {
        if self.threshold == 0 || self.threshold > self.keys.len() {
            return Err(Error::InvalidThreshold);
        }

        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }

        let keys: Vec<Policy<DescriptorPublicKey>> =
            self.keys.into_iter().map(Policy::Key).collect();
        Ok(Policy::And(vec![
            Policy::Threshold(self.threshold, keys),
            match self.timelock {
                Locktime::After(after) => Policy::After(after.into()),
                Locktime::Older(older) => Policy::Older(older),
            },
        ]))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum PolicyTemplate {
    Multisig {
        threshold: usize,
        keys: Vec<DescriptorPublicKey>,
    },
    Recovery {
        my_key: DescriptorPublicKey,
        recovery: RecoveryTemplate,
    },
    Hold {
        my_key: DescriptorPublicKey,
        timelock: Locktime,
    },
    Decaying {
        start_threshold: usize,
        keys: Vec<DescriptorPublicKey>,
        time: DecayingTime,
    },
}

impl PolicyTemplate {
    #[inline]
    pub fn multisig(threshold: usize, keys: Vec<DescriptorPublicKey>) -> Self {
        Self::Multisig { threshold, keys }
    }

    #[inline]
    pub fn recovery(my_key: DescriptorPublicKey, recovery: RecoveryTemplate) -> Self {
        Self::Recovery { my_key, recovery }
    }

    #[inline]
    pub fn hold(my_key: DescriptorPublicKey, timelock: Locktime) -> Self {
        Self::Hold { my_key, timelock }
    }

    #[inline]
    pub fn decaying(
        start_threshold: usize,
        keys: Vec<DescriptorPublicKey>,
        time: DecayingTime,
    ) -> Self {
        Self::Decaying {
            start_threshold,
            keys,
            time,
        }
    }

    pub fn build(self) -> Result<Policy<DescriptorPublicKey>, Error> {
        match self {
            Self::Multisig { threshold, keys } => {
                if threshold == 0 || threshold > keys.len() {
                    return Err(Error::InvalidThreshold);
                }

                if keys.is_empty() {
                    return Err(Error::NoKeys);
                }

                let keys: Vec<Policy<DescriptorPublicKey>> =
                    keys.into_iter().map(Policy::Key).collect();
                Ok(Policy::Threshold(threshold, keys))
            }
            Self::Recovery { my_key, recovery } => Ok(Policy::Or(vec![
                (1, Policy::Key(my_key.clone())),
                (1, recovery.build()?),
            ])),
            Self::Hold { my_key, timelock } => Ok(Policy::And(vec![
                Policy::Key(my_key.clone()),
                match timelock {
                    Locktime::After(after) => Policy::After(after.into()),
                    Locktime::Older(older) => Policy::Older(older),
                },
            ])),
            Self::Decaying {
                start_threshold,
                keys,
                time,
            } => {
                if start_threshold == 0 || start_threshold > keys.len() {
                    return Err(Error::InvalidThreshold);
                }

                if keys.is_empty() {
                    return Err(Error::NoKeys);
                }

                let mut list: Vec<Policy<DescriptorPublicKey>> =
                    keys.into_iter().map(Policy::Key).collect();

                match time {
                    DecayingTime::Single(timelock) => match timelock {
                        Locktime::After(after) => list.push(Policy::After(after.into())),
                        Locktime::Older(older) => list.push(Policy::Older(older)),
                    },
                    DecayingTime::Multiple(timelocks) => {
                        for timelock in timelocks.into_iter() {
                            match timelock {
                                Locktime::After(after) => list.push(Policy::After(after.into())),
                                Locktime::Older(older) => list.push(Policy::Older(older)),
                            }
                        }
                    }
                }

                Ok(Policy::Threshold(start_threshold, list))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_multisig_template() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();

        // 1 of 2
        let template = PolicyTemplate::multisig(1, vec![desc1.clone(), desc2.clone()]);
        assert_eq!(template.build().unwrap().to_string(), String::from("thresh(1,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))"));

        // 2 of 2
        let template = PolicyTemplate::multisig(2, vec![desc1, desc2]);
        assert_eq!(template.build().unwrap().to_string(), String::from("thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))"));
    }

    #[test]
    fn test_invalid_multisig_template() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();

        let template = PolicyTemplate::multisig(0, vec![desc1.clone(), desc2.clone()]);
        assert_eq!(template.build().unwrap_err(), Error::InvalidThreshold);

        let template = PolicyTemplate::multisig(3, vec![desc1, desc2]);
        assert_eq!(template.build().unwrap_err(), Error::InvalidThreshold);
    }

    #[test]
    fn test_social_recovery_template() {
        // My Key
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();

        // Recovery keys
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
        let desc3 = DescriptorPublicKey::from_str("[f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*").unwrap();

        let older = Sequence(6);
        let recovery = RecoveryTemplate::new(2, vec![desc2, desc3], Locktime::Older(older));
        let template = PolicyTemplate::recovery(desc1, recovery);
        assert_eq!(template.build().unwrap().to_string(), String::from("or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),older(6)))"));
    }

    #[test]
    fn test_inheritance_template() {
        // My Key
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();

        // Recovery keys
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
        let desc3 = DescriptorPublicKey::from_str("[f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*").unwrap();

        let after = AbsoluteLockTime::from_height(840_000).unwrap();
        let recovery = RecoveryTemplate::new(2, vec![desc2, desc3], Locktime::After(after));
        let template = PolicyTemplate::recovery(desc1, recovery);
        assert_eq!(template.build().unwrap().to_string(), String::from("or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),after(840000)))"));
    }

    #[test]
    fn test_hold_template() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let older = Locktime::Older(Sequence(10_000));
        let template = PolicyTemplate::hold(desc1, older);
        assert_eq!(template.build().unwrap().to_string(), String::from("and(pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),older(10000))"));
    }
}
