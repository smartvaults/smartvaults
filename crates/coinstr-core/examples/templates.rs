// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::miniscript::DescriptorPublicKey;
use coinstr_core::{AbsoluteLockTime, PolicyTemplate, RecoveryTemplate, Sequence};

fn main() {
    // Descriptors
    let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
    let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();
    let desc3 = DescriptorPublicKey::from_str("[f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*").unwrap();

    // Multisig template (2 of 2)
    let template = PolicyTemplate::multisig(1, vec![desc1.clone(), desc2.clone()]);
    println!(
        "Multisig (2of2): {}\n",
        template.build().unwrap().to_string()
    );

    // Social Recovery
    let older = Sequence(6);
    let recovery = RecoveryTemplate::social_recovery(2, vec![desc2.clone(), desc3.clone()], older);
    let template = PolicyTemplate::recovery(desc1.clone(), recovery);
    println!(
        "Social Recovery: {}\n",
        template.build().unwrap().to_string()
    );

    // Inheritance
    let after = AbsoluteLockTime::from_height(840_000).unwrap();
    let recovery = RecoveryTemplate::inheritance(2, vec![desc2, desc3], after);
    let template = PolicyTemplate::recovery(desc1.clone(), recovery);
    println!("Inheritance: {}\n", template.build().unwrap().to_string());

    // Hold
    let older = Sequence(10_000);
    let template = PolicyTemplate::hold(desc1, older);
    println!("Hold: {}", template.build().unwrap().to_string());
}
