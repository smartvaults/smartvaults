## Quick start

```
cargo build --release
./target/release/coinstr-cli
```

```
Using nostr to coordinate Bitcoin spending policy signatures and multi-custody

Usage: coinstr-cli [OPTIONS] <COMMAND>

Commands:
  generate     Generate new keychain
  restore      Restore keychain
  list         List keychains
  inspect      Inspect bitcoin and nostr keys
  save-policy  Save policy
  spend        Create a spending proposal
  approve      Approve a spending proposal
  broadcast    Combine and broadcast the transaction
  get          Get data about policies and proposals
  delete       Delete
  setting      Setting
  help         Print this message or the help of the given subcommand(s)

Options:
  -n, --network <NETWORK>  Network [default: bitcoin] [possible values: bitcoin, testnet, signet, regtest]
  -r, --relay <RELAY>      Relay [default: wss://relay.rip]
  -h, --help               Print help
  -V, --version            Print version

```

## Get policies
```bash
COINSTR_PASSWORD=test ./target/release/coinstr-cli get policies lee
```

```
+---+------------------------------------------------------------------+--------------------------------------+-------------------------------------------------------+
| # | ID                                                               | Name                                 | Description                                           |
+===+==================================================================+======================================+=======================================================+
| 1 | fa58f0abd28e01acbe1712f54cdd759dec53aa2d28e6437a66d30ad8462c55f7 | 2 of 4 policy for waterwell          | 2 of 4 policy for waterwell short term spending needs |
+---+------------------------------------------------------------------+--------------------------------------+-------------------------------------------------------+
| 2 | 4f13ae0ce31cd256a4b3b84ddc0c76c2399e8bbd3e9659b6bb81e83f88fbea82 | 6 of 10 with 1 year wait             | Requires 6 out of 10 with a 1 year waiting period     |
+---+------------------------------------------------------------------+--------------------------------------+-------------------------------------------------------+
| 3 | dafc2f23c8a863e3ddb8eed08992c880d19d7e82a0ed76a9ef722a7b36f8c87f | 3 of 5 board member initial adopters | Requires 3 of 5 board members                         |
+---+------------------------------------------------------------------+--------------------------------------+-------------------------------------------------------+
```

## Get policy
```
COINSTR_PASSWORD=test ./target/release/coinstr-cli --network testnet get policy lee 4f13ae0ce31cd256a4b3b84ddc0c76c2399e8bbd3e9659b6bb81e83f88fbea82
```

```
Policy
- ID: 4f13ae0ce31cd256a4b3b84ddc0c76c2399e8bbd3e9659b6bb81e83f88fbea82
- Name: 6 of 10 with 1 year wait
- Description: Requires 6 out of 10 with a 1 year waiting period
- Descriptor
└── id -> jw7y2gax
    └── 👑 Threshold Condition   : 1 of 2 
        ├── id -> wt8mt054
        │   └── 🔑 Schnorr Sig of  <xonly-pk:c42c4c164f56211e393c0d72adf9c7de7ac2af63b0a49bbc4c7f7c144d2a65df>
        └── id -> g7vvrmru
            └── 👑 Threshold Condition   : 2 of 2 
                ├── id -> j6t74585
                │   └── 🤝 MultiSig  :  6 of 10
                │       ├── 🔑 <xonly-pk:0dd81025a7b83c6f432b7afe1591417a4074b2e64b9824990a4f5709eb566320>
                │       ├── 🔑 <xonly-pk:101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df>
                │       ├── 🔑 <xonly-pk:41be80424dfb9b33d66ea4f5369cc6b10afaa1b0b167ad7b8112fd6848faa32e>
                │       ├── 🔑 <xonly-pk:51fd73484c435388b4a276a86b7a6888d83c074e91621e10736f39f3dc77284f>
                │       ├── 🔑 <xonly-pk:ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d>
                │       ├── 🔑 <xonly-pk:19b5decafadedc2a318e731c248a3fa16a6e5f7e8161ad99767c5fea502342ed>
                │       ├── 🔑 <xonly-pk:d484718041e84f42889219d850bb7f17805a04ca6b70e20d4a12ab3e959243e2>
                │       ├── 🔑 <xonly-pk:efc3f1bd307c2b3374ee2c72d3cb1213238cb1ac4a338a719335d6f256f3d901>
                │       ├── 🔑 <xonly-pk:e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc>
                │       └── 🔑 <xonly-pk:c04e8da91853b7fd215102e6aa48477d8e1ba6b3c16902371a153d3784a1b0f7>
                └── id -> unzg4a67
                    └── ⏰ Absolute Timelock of  52560

Balances
- Immature            	: 0 sats
- Trusted pending     	: 0 sats
- Untrusted pending   	: 0 sats
- Confirmed           	: 3 000 sats

Deposit address: tb1pqt7zfuvek8z2ymgjzahftq04sd7xj7rujyjd69acvxl9n4f95alqmtkuv6
```
## Get information about a key
```bash
COINSTR_PASSWORD=test ./target/release/coinstr-cli inspect lee
```

```
Mnemonic: volume lyrics health attitude hidden enable afford grid ozone rotate wash blood

Nostr
 Bech32 Keys
  Public   : npub1aff8upvht8fk3f2j2vnsg48936wkunlzaxxnqttwqxppl2tnykwsahwngp 
  Private  : nsec18lkp320pjm7n5eqhk3066uq9akermpffedqa3trn3n7a054h2ems37v3ar 
 Hex Keys
  Public   : ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d 
  Private  : 3fec18a9e196fd3a6417b45fad7005edb23d8529cb41d8ac738cfdd7d2b75677 
  Normalized Public   : 02ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d 

Bitcoin
  Root Private Key: xprv9s21ZrQH143K4Ph9z7CdHHcMzGAScvgh8Y1zsYkzbhEKqe1wKaotRm22y3h8C2QRd4RCoo3V59ygTpq2NtRKTth4Hgoh8sXUkiTSXouvF4n
  Extended Pub Key: xpub661MyMwAqRbcGsmd68jdeRZ6YHzw2PQYVkwbfwAcA2mJiSM5s888yZLWpKyBHUQTK5b7RkLp2quDwtq4kn95VBFsa7TjVZJNEHQijWMadFN
  Output Descriptor: tr([7b264e11/86'/0'/0']xpub6DMLuW1nPtGMxBeEujHwz57L7UMxfLCCdhxnN3dsjaJQTvdvCzhp2oikQGZ6qhewrcb9viB66WF51NUDbAEmSTgyfvmXQc5K8RzAip7nJ1p/0/*)#7td2nxg4
  Change Descriptor: tr([7b264e11/86'/0'/0']xpub6DMLuW1nPtGMxBeEujHwz57L7UMxfLCCdhxnN3dsjaJQTvdvCzhp2oikQGZ6qhewrcb9viB66WF51NUDbAEmSTgyfvmXQc5K8RzAip7nJ1p/1/*)#0lgtwncd
  Ext Address 1: bc1pp97fxzg2gj90k2kzh9pkygn5kspfv49xkjjwn7cgzvafv26ehvqq548kh4
  Ext Address 2: bc1plperhnla7w5ay8dzmex5fwdja58nhg5vuzvv4t607s242v92j39qqs2hs2
  Change Address: bc1pw75gajlhzycffcd49dhzy73z0fg5k348km2jsh27fl0qahmnkaqstkq0cl

```
## Coinstr Product Video Playlist
[![Watch the video](https://img.youtube.com/vi/_-K8K_76K24/default.jpg)](https://www.youtube.com/playlist?list=PLQvYD9hYsl07Iq8WkAk8sfrGC8jkWZZA4)

## End-to-end spend tutorial
[![Watch the video](https://img.youtube.com/vi/jW5_6kZWuWU/default.jpg)](https://www.youtube.com/watch?v=jW5_6kZWuWU)