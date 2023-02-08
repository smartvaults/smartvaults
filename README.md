
```
Using nostr to coordinate Bitcoin spending policy signatures and multi-custody

Usage: coinstr <COMMAND>

Commands:
  generate   Generates random account(s)
  subscribe  Subscribe to nostr events
  publish    Publish a nostr event
  inspect    Inspect a mnenonic for validity and print bitcoin and nostr keys
  convert    Convert between hex and bech32 format keys
  balance    Find the balance for a bitcoin descriptor
  get        Get things
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Quick start

```
cargo build --release
```

### Get list of known users
```bash
± |main U:1 ✗| → ./target/release/coinstr get users
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
  Secret Key (HEX)    : 0e1db7418df1c6453ce42e7f4507b8823fc23e86e1f4f33d7fafc83d366e6e97 
  Secret Key (bech32) : nsec1pcwmwsvd78ry208y9el52pacsgluy05xu860x0tl4lyr6dnwd6tsdak7nt 
  Public Key (HEX)    : 0330e85095c0e622b73160a30858df7180e07b1faaa49483369cd4d95eeac54d0f 
  X Only Public Key   : 30e85095c0e622b73160a30858df7180e07b1faaa49483369cd4d95eeac54d0f 
  Public Key (bech32) : npub1xr59p9wquc3twvtq5vy93hm3srs8k8a25j2gxd5u6nv4a6k9f58schcx7v 

Bitcoin Configuration
  Output Descriptor   : tr([9b5d4149/44'/0'/0']tpubDDtTEcifwjX3Ri5g8WUTGxAqst9BqRynWCNfM69u3wfcxoPAX9kYhzCF9peMsSRuuSi1aFLWdj8GSjPavgfZQcTETM85obokxHR1TLCsNK2/0/*)#8utyfc84
  Address             : tb1prfek6jnap5yjlj4m6wsjwq3wg4hrxee59w8lj9ajl60xlmqqxhqsevv5zf
  Address             : tb1p7ujlfhkgv2j4zvjd0u4wpgtne2k96r7k50qn63wxkcqecp0c4t8s62jrwj

Bitcoin Balances
  Immature            : 0 
  Trusted Pending     : 0 
  Untrusted Pending   : 0 
  Confirmed           : 2000 
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