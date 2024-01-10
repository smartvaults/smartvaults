# 0.4.0 (2024/01)

## Key Agency
- Key agents are trusted parties that hold custody of one or more keys on a multisig vault. See our [Key Agency FAQ](https://smartvaults.io)
- Key agents can use Smart Vaults to register, configure, and share their signers (x-pubs) with users.
- Key agents can specify their price in USD or sats, in price per signature and/or an annual flat rate or basis points.
- Users discover key agents through the Key Agent catalog, and they collaboratively build vaults.
- Key agent fees are auto-calculated and pro-rated, facilitating processing of micro-transactions to pay key agent fees during low chain-fee time periods. 
- Contact us via [Telegram](https://t.me/+I3B8_4tz7sMwZjVh) to become a verified key agent or try it out on the [testnet](https://smartvaults.dev). 

## Vault Collaboration
- Added vault invite and join flows, including non-participant ‘watchers’
- Ability to send encrypted chat messages among vault participants. 
- Group chat messaging attached to spending proposals.

## Usability Improvements
- Mobile push notifications (blinded/end-to-end encrypted) when a user shares a key, participates in a vault, and when a proposal is made, approved, or broadcast. 
- UTXO Management: label receiving addresses and UTXOs, select UTXOs as inputs on transactions.
- Policy path selection: select the tap tree script path when building a transaction (enables more flexible and dynamic miniscript policies)
- Added ability to view vault balances in fiat (web)

## Hardware Wallet Support 
- Coldcard (EDGE firmware)

## MiniTapscript template vaults for: 
- Decaying Multisig
- Collaborative Custody
- Hold Lock
- Social Recovery

