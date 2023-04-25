
# Step by Step - 2 of 2 Multisig
This step by step guide shows how to create a 2 of 2 multisignature Bitcoin transaction using `coinstr-cli`. 

## Step 1. Setup Keys
Create keys and save them to keychains. You can use the keychain names of `alice-2of2` and `bob-2of2`. A password to encrypt your keychain **is** required, but a passphrase (which modifies the key) is not required.

> NOTE: Mnemonics will be printed but they don't need to be saved for this tutorial.
```
./target/release/coinstr-cli --network testnet generate alice-2of2
./target/release/coinstr-cli --network testnet generate bob-2of2
```

## Step 2: Create the 2-of-2 Policy
Using miniscript, we will create a 2 of 2 multisignature policy that follows this format: 
```
thresh(2,pk(<ALICE_PUBKEY>),pk(<BOB_PUBKEY>))
```

Use the **Normalized Public** key from the output of Coinstr's `inspect` command to replace the variable in the miniscript above. For example, 

```
./target/release/coinstr-cli --network testnet inspect alice-2of2
```

Produces the following output (your info will be different): 
```
Mnemonic: bronze craft exotic elite because boat spirit false sustain animal expect solar

Nostr
 Bech32 Keys
  Public   : npub1a6e6w5ka0p3uprncxs5swmgw7vyw7w32g8pd7st6s48g9pangcfqfq6nn3 
  Private  : nsec19nawhfc7w770s36y6w8e99watffquv2ekygdf57gdhmpgf40y4nstwcn6s 
 Hex Keys
  Public   : eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612 
  Private  : 2cfaeba71e77bcf84744d38f9295dd5a520e3159b110d4d3c86df61426af2567 
  Normalized Public   : 03eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612 

Bitcoin
  Root Private Key: tprv8ZgxMBicQKsPdWz8gEZNAuyeeMLa9ugBRfbUtGCADTns3cLLNmL74gvuh2v9VymxMfLM3X3U6zorFX9VqhL4z8UgSVsGyHBc3z14FX3xrva
  Extended Pub Key: tpubD6NzVbkrYhZ4Wz1vZtDxaKdmDNrWKEs5zyCGAnETdjbFt6b71A9hFBYmsBXejM5Gj1e8kWvPhUCHNTv87uFgMKKag7824J8iey7tCxezxUp
  Output Descriptor: tr([c7bafad9/86'/1'/0']tpubDCTkCZYFTVtoUxcBnSJ96zDg98wGUNVrtHDL9Z88CqqoQmbWwMNZydbCUttd6sgcsBZYdhV4XvwjXbq5WinYnW6utJTHXvPVGWJVz99a9Wc/0/*)#gc7e5fg2
  Change Descriptor: tr([c7bafad9/86'/1'/0']tpubDCTkCZYFTVtoUxcBnSJ96zDg98wGUNVrtHDL9Z88CqqoQmbWwMNZydbCUttd6sgcsBZYdhV4XvwjXbq5WinYnW6utJTHXvPVGWJVz99a9Wc/1/*)#evmcfucj
  Ext Address 1: tb1pa3f2r9yk3rz9ucdlce8qqjk3zmn5r9gmvw0wvrj6pel5ptn3f4tqy7wpfk
  Ext Address 2: tb1pdkvplsmnstdewr39ncpaczslqnjfmr0wpx7acawf985hgm73n84s7fcg68
  Change Address: tb1p3ey4dltfjx8dantgwgvdzah2940ram0qpalgkw3adrcu46847ftq2jgcxq
```

For example, Alice's Normalized Public key from above is `03eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612`. 

In my keys, Bob's Normalized Public key is 
`025eace63cc5d93fb883082d30ccfbe43f16bbe869a2f1f0858a86ed6fa0475d52`. 

So, the miniscript for our 2 of 2 policy is (your keys will be different):
```
thresh(2,pk(03eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612),pk(025eace63cc5d93fb883082d30ccfbe43f16bbe869a2f1f0858a86ed6fa0475d52))
```

## Step 3: Save the Policy 
```
./target/release/coinstr-cli save-policy alice-2of2 \
    "Multisig 2 of 2" \
    "Testing multisig as part of the Coinstr demo" \
    "thresh(2,pk(03eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612),pk(025eace63cc5d93fb883082d30ccfbe43f16bbe869a2f1f0858a86ed6fa0475d52))"
```

Now you can review the saved policies for alice using the following command: 
```
./target/release/coinstr-cli --network testnet get policies alice-2of2
```

Produces: 
```
+---+------------------------------------------------------------------+-----------------+----------------------------------------------+
| # | ID                                                               | Name            | Description                                  |
+===+==================================================================+=================+==============================================+
| 1 | 0a24f8f6ff8142014cda6db00abc09d20508f0f0db03b13b26feb675e7fec0f0 | Multisig 2 of 2 | Testing multisig as part of the Coinstr demo |
+---+------------------------------------------------------------------+-----------------+----------------------------------------------+
```

You can see the details of the policy by calling `get policy`: 
```
./target/release/coinstr-cli --network testnet get policy \
    alice-2of2 \
    0a24f8f6ff8142014cda6db00abc09d20508f0f0db03b13b26feb675e7fec0f0
```

> NOTE: Bob has the same policy saved into his list.

Produces the following output: 
```
- ID: 0a24f8f6ff8142014cda6db00abc09d20508f0f0db03b13b26feb675e7fec0f0
- Name: Multisig 2 of 2
- Description: Testing multisig as part of the Coinstr demo
- Descriptor
└── id -> al0awsk0
    └── 👑 Threshold Condition   : 1 of 2 
        ├── id -> qazxleng
        │   └── 🔑 Schnorr Sig of  <xonly-pk:df0753f2ae3ca1dc3d5df6abadccb636d48c32be2860688d9c1eb0d7e8fbea2b>
        └── id -> ns3ern48
            └── 🤝 MultiSig  :  2 of 2
                ├── 🔑 <pk:03eeb3a752dd7863c08e783429076d0ef308ef3a2a41c2df417a854e8287b34612>
                └── 🔑 <pk:025eace63cc5d93fb883082d30ccfbe43f16bbe869a2f1f0858a86ed6fa0475d52>

Balances
- Immature            	: 0 sat
- Trusted pending     	: 0 sat
- Untrusted pending   	: 0 sat
- Confirmed           	: 0 sat

Deposit address: tb1ph64hcng7egq67qm9q7k2n805qf86llxeup9l86n4yv6sszxmn5uqjnwmdw
```

## Step 4: Get Testnet BTC from Faucet
Use the [testnet bitcoin faucet](https://testnet-faucet.com/btc-testnet/) to request BTC for our policy. The deposit address is at the bottom of the output above.

## Step 5: Generate a Spend Proposal
We will create the spend proposal from Alice's perspective. to create a spend proposal: 
```
Usage: coinstr-cli spend <NAME> <POLICY_ID> <TO_ADDRESS> <AMOUNT> <DESCRIPTION>
```
```
./target/release/coinstr-cli --network testnet spend \
    alice-2of2 \
    0a24f8f6ff8142014cda6db00abc09d20508f0f0db03b13b26feb675e7fec0f0 \
    mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78 \
    1000 \
    "Send back to the faucet"
```
You can now view the spend proposal:
```
./target/release/coinstr-cli get proposals alice-2of2
```
Create the below table: 
```
+---+------------------------------------------------------------------+-----------+-------------------------+------------------------------------+------------+
| # | ID                                                               | Policy ID | Description             | Address                            | Amount     |
+===+==================================================================+===========+=========================+====================================+============+
| 1 | c000d26a0e79a37df6235af25230acb43a22460f51e8b00057b481f1ad55a6b6 | 0a24f8f6f | Send back to the faucet | mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78 | 1 000 sat |
+---+------------------------------------------------------------------+-----------+-------------------------+------------------------------------+------------+
```

## Step 6: Approve a Spend Proposal
Now we need to approve the proposal from both Alice and Bob's perspective.
```
./target/release/coinstr-cli --network testnet approve \
    alice-2of2 \
    c000d26a0e79a37df6235af25230acb43a22460f51e8b00057b481f1ad55a6b6

./target/release/coinstr-cli --network testnet approve \
    bob-2of2 \
    c000d26a0e79a37df6235af25230acb43a22460f51e8b00057b481f1ad55a6b6
```

> NOTE: if you try to broadcast the transaction before it is finalized, you will get an error such as `PSBT not finalized: [InputError(CouldNotSatisfyTr, 0), InputError(CouldNotSatisfyTr, 1)]`.


## Step 7: Broadcast the Transaction
```
coinstr-cli --network testnet broadcast \
    alice-2of2 \
    c000d26a0e79a37df6235af25230acb43a22460f51e8b00057b481f1ad55a6b6
```

You will get a transaction-id that you can view with a block explorer: 
```
Transaction 2b4226bd85fb32a833bfbde59728e21c2574d93a736d636ca689572c3731808a broadcasted
```

https://blockstream.info/testnet/tx/2b4226bd85fb32a833bfbde59728e21c2574d93a736d636ca689572c3731808a





