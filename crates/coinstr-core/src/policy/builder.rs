// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::bitcoin::XOnlyPublicKey;
use bdk::miniscript::policy::concrete::Policy;
use bdk::miniscript::DescriptorPublicKey;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid threshold")]
    InvalidThreshold,
    #[error("not keys")]
    NoKeys,
}

pub fn n_of_m_multisig(required_sig: usize, keys: Vec<XOnlyPublicKey>) -> Result<String, Error> {
    if required_sig == 0 {
        return Err(Error::InvalidThreshold);
    }

    if keys.is_empty() {
        return Err(Error::NoKeys);
    }

    let keys: Vec<Policy<XOnlyPublicKey>> = keys.into_iter().map(Policy::Key).collect();
    Ok(Policy::Threshold(required_sig, keys).to_string())
}

pub fn n_of_m_ext_multisig(
    required_sig: usize,
    extended_descs: Vec<DescriptorPublicKey>,
) -> Result<String, Error> {
    if required_sig == 0 {
        return Err(Error::InvalidThreshold);
    }

    if extended_descs.is_empty() {
        return Err(Error::NoKeys);
    }

    let keys: Vec<Policy<DescriptorPublicKey>> =
        extended_descs.into_iter().map(Policy::Key).collect();
    Ok(Policy::Threshold(required_sig, keys).to_string())
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_1_of_2_ext_multisig() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
        assert_eq!(n_of_m_ext_multisig(1, vec![desc1, desc2]).unwrap(), String::from("thresh(1,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))"))
    }

    #[test]
    fn test_2_of_2_ext_multisig() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
        assert_eq!(n_of_m_ext_multisig(2, vec![desc1, desc2]).unwrap(), String::from("thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))"))
    }

    #[test]
    fn test_invalid_policy() {
        let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
        assert_eq!(
            n_of_m_ext_multisig(0, vec![desc1, desc2]).unwrap_err(),
            Error::InvalidThreshold
        )
    }
}
