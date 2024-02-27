// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::{
    AbsoluteLockTime, DecayingTime, Locktime, PolicyTemplate, RecoveryTemplate, Sequence,
};

fn main() {
    // Descriptors
    let desc1 = DescriptorPublicKey::from_str("[7356e457/48'/1'/0'/3']tpubDFjrNfaRKF1vvzELxYgrSvtHZPeyXBJZpHQkWsuVqe9WK7ndRESHZpSsAzPJdd2Bsyer9wcjoCbKQxHTfDcR23ZUjCezaGjRjCjMVfTshrZ/0/*").unwrap();
    let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/48'/1'/0'/3']tpubDEq8TpwmaKaKF5hTZPhpxiEEXcAvnLi6ksd81XYGFFyVfCtv5m9svkZAZ41PhHuMuajkrg1yKKtUzVtqALP87M9df2FYGYWf2coYAnHc1AP/0/*").unwrap();
    let desc3 = DescriptorPublicKey::from_str("[f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*").unwrap();

    // Multisig template (2 of 2)
    let template = PolicyTemplate::multisig(2, vec![desc1.clone(), desc2.clone()]);
    println!(
        "Multisig (2of2): {}\n",
        template.build().unwrap().to_string()
    );

    // Recovery template (older)
    let older = Locktime::Older(Sequence(6));
    let recovery = RecoveryTemplate::new(2, vec![desc2.clone(), desc3.clone()], older);
    let template = PolicyTemplate::recovery(desc1.clone(), recovery);
    println!(
        "Recovery (older): {}\n",
        template.build().unwrap().to_string()
    );

    // Recovery template (after)
    let after = Locktime::After(AbsoluteLockTime::from_height(840_000).unwrap());
    let recovery = RecoveryTemplate::new(2, vec![desc2.clone(), desc3.clone()], after);
    let template = PolicyTemplate::recovery(desc1.clone(), recovery);
    println!(
        "Recovery (after): {}\n",
        template.build().unwrap().to_string()
    );

    // Hold
    let older = Locktime::Older(Sequence(10_000));
    let template = PolicyTemplate::hold(desc1.clone(), older);
    println!("Hold: {}", template.build().unwrap().to_string());

    // Decaying
    let template = PolicyTemplate::decaying(
        3,
        vec![desc1.clone(), desc2.clone(), desc3.clone()],
        DecayingTime::Single(Locktime::Older(Sequence(2))),
    );
    println!("Decaying: {}\n", template.build().unwrap().to_string());
}
