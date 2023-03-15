## Quick start

```
cargo build --release
./target/release/coinstr
```

```
Using nostr to coordinate Bitcoin spending policy signatures and multi-custody

Usage: coinstr <COMMAND>

Commands:
  generate     Generates random account(s)
  subscribe    Subscribe to nostr events
  publish      Publish a nostr event
  inspect      Inspect a mnenonic for validity and print bitcoin and nostr keys
  convert      Convert between hex and bech32 format keys
  balance      Find the balance for a bitcoin descriptor
  save-policy  Save policy
  get          Get data about events and users
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Get Event
```bash 
./target/release/coinstr get event d3a421ae9cde2a530429867db0923fcfd5812dde84bb789169cd99b1d53d236a
```

## Show spending policies
```bash
→ cargo test policy -- --nocapture
running 2 tests

Coinstr Policy
Name            : 💸 My testing policy
Description     : A policy for testing Alice and Bob multisig

Coinstr Policy
Name            : 💸 My testing policy
Description     : A policy for testing Alice and Bob multisig
💸 My testing policy
└── 🆔 ktrzwzm6
    └── 🎚️ Threshold Condition    : 1 of 2 
        ├── 🆔 96d6dvge
        │   └── 🔑 Schnorr Sig of <fingerprint:06d1e3e7>
        └── 🆔 460alevg
            └── 🔑 Schnorr Sig of <fingerprint:ca0b6651>

test policy::tests::build_multisig_policy ... ok
💸 My testing policy
└── 🆔 ng5yfwlw
    └── 🎚️ Threshold Condition    : 2 of 2 
        ├── 🆔 nk7jnzl3
        │   └── ✍️ ECDSA Sig of <pk:02e5d000a7ea6d5c577245bd8e8727d0b57f12d1d06bb8c7266df3e1ff22f326e9>
        └── 🆔 kxkjs274
            └── 🎚️ Threshold Condition    : 1 of 2 
                ├── 🆔 hn0csay5
                │   └── ✍️ ECDSA Sig of <pk:032c9bf7a1a5074d790b9ff7f4b6f9595f4ff61d132da0d234ce47c69e9f2e5f89>
                └── 🆔 hwm4g28x
                    └── ⏳ Relative Timelock of 12960

test policy::tests::build_with_descriptor ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.23s
```

### Get list of known users
```bash
→ ./target/release/coinstr get users
Alice
Bob
Charlie
David
Erika
```

### Get information about a user
```bash
→ ./target/release/coinstr get user --user alice
Name       : Alice

Mnemonic   : "carry surface crater rude auction ritual banana elder shuffle much wonder decrease" 
Passphrase : "oy+hB/qeJ1AasCCR" 

Nostr Configuration
 Bech32 Keys
  Public   : npub1xr59p9wquc3twvtq5vy93hm3srs8k8a25j2gxd5u6nv4a6k9f58schcx7v 
  Private  : nsec1pcwmwsvd78ry208y9el52pacsgluy05xu860x0tl4lyr6dnwd6tsdak7nt 
 Hex Keys
  Public   : 30e85095c0e622b73160a30858df7180e07b1faaa49483369cd4d95eeac54d0f 
  Private  : 0e1db7418df1c6453ce42e7f4507b8823fc23e86e1f4f33d7fafc83d366e6e97 


Bitcoin Configuration
  Root Private Key      : tprv8ZgxMBicQKsPeFd9cajKjGekZW5wDXq2e1vpKToJvZMqjyNkMqmr7exPFUbJ92YxSkqL4w19HpuzYkVYvc4n4pvySBmJfsawS7Seb8FzuNJ
  Root Public Key       : tpubD6NzVbkrYhZ4XiewWEPv8gJs8XbsNs1wDKXbbyqcLqAEaTdWzEbSJ9aFRamjrj3RQKyZ2Q848BkMxyt6J6e36Y14ga6Et7suFXk3RKFqEaA
  Output Descriptor     : tr([9b5d4149/86'/0'/0']tpubDDfNLjZpqGcbyjiSzxxbvTRqvySNkCQKKDJHXkJPZCKQPVsVX9fcuvkd65MU3oyRmqgzpzvuEUxe6zstCCDP2ogHn5ModwnrxP4cdWLFdc3/0/*)#2azlv5fk
  Change Descriptor     : tr([9b5d4149/86'/0'/0']tpubDDfNLjZpqGcbyjiSzxxbvTRqvySNkCQKKDJHXkJPZCKQPVsVX9fcuvkd65MU3oyRmqgzpzvuEUxe6zstCCDP2ogHn5ModwnrxP4cdWLFdc3/1/*)#mf873pew
  Ext Address 1         : tb1p50q6uztqeg42gjqga0gtkax7kl2vd72v2mwqytn754s768w3rvlq09w3hc
  Ext Address 2         : tb1pkzdfxvwp2ehvjasrzh78vvstk9smwx204naf58g0dye2p7s9hkgs060u7e
  Change Address        : tb1p2y8glskt8wz288suzdhm6vs7nkwwxwtmlc00gd5gdd8qzqjz8gusuy0vkq

Bitcoin Balances
  Immature              : 0 
  Trusted Pending       : 0 
  Untrusted Pending     : 0 
  Confirmed             : 4000 
```

### Generate keys
```bash
→ ./target/release/coinstr generate -p "my passphrase"

Generating account 1 of 1

Mnemonic   : "good layer theme chronic maid canyon credit trend visit rent ahead destroy" 
Passphrase : "my passphrase" 

Nostr Configuration
  Secret Key (HEX)    : f99a0ee1abcfe6b1ca8bd32efcd59bbe42f98b41c4b6b8967669f1c1ec2f2711 
  Secret Key (bech32) : nsec1lxdqacdtelntrj5t6vh0e4vmhep0nz6pcjmt39nkd8curmp0yugss3s2tk 
  Public Key (HEX)    : 0334a52d14b55bd8b85cc2787c2cc206ea6e0c8f9fc4d80137bd4d44b26a73981e 
  X Only Public Key   : 34a52d14b55bd8b85cc2787c2cc206ea6e0c8f9fc4d80137bd4d44b26a73981e 
  Public Key (bech32) : npub1xjjj6994t0vtshxz0p7zessxafhqerulcnvqzdaaf4zty6nnnq0qjy0xj5 

Bitcoin Configuration
  Output Descriptor   : tr([323ef8a3/44'/0'/0']tpubDC3mc2hbH3tn2UfbG7JsAL5WMkapHudFw8d2BP1mCMrgRXptx29axoo8H26uyaxcpwQAtWA8BgsLMJKnD4kS9tJr4X7t4kaMQy1bHsGtGLm/0/*)#w057gvdm
  Address             : tb1p4zv8gd85mqxft8m7l7h7tnezvlsdd9efr8y9akxxv43t3v7q8efs0tvx7g
  Address             : tb1pwl4y2yydjehgluqryx98j2pmvkasxme5ky59c09ktk8zhq0tfedsuu9emw

Bitcoin Balances
  Immature            : 0 
  Trusted Pending     : 0 
  Untrusted Pending   : 0 
  Confirmed           : 0 
```

## Setup local nostr relay
> WARNING: `nostr-rs-relay` has some known issues; recommend to use `strfry` or `wss://relay.rip`
```
git clone git@github.com:scsibug/nostr-rs-relay.git
cd nostr-rs-relay
cargo build --release
RUST_LOG=warn,nostr_rs_relay=info ./target/release/nostr-rs-relay
```