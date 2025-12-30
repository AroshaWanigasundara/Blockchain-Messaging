# Pallet Messaging (Secure Messaging)

A decentralized, encrypted messaging system pallet that uses blockchain for verification and signaling but stores message content off-chain for privacy.

## Overview

This pallet establishes:
- **On-chain key registry**: Stores public keys for identity verification
- **Message hash signaling**: Records hashes of encrypted messages for verification only
- **Spam protection**: Anti-spam bonds that can be refunded or forfeited

## Features

### Storage Items
- **Profiles**: Map of `AccountId → PublicKey` for on-chain identity
- **MessageHashes**: Map of `MessageId → Hash` for message verification
- **Bonds**: Map of `AccountId → Balance` for spam protection bonds
- **Contacts**: Double map of approved contacts for each account

### Extrinsics
1. `register_profile(public_key)` - Register a public key
2. `send_message_hash(msg_id, msg_hash)` - Store a message hash on-chain (requires registration + bond)
3. `approve_contact(contact)` - Approve a contact
4. `remove_contact(contact)` - Remove an approved contact
5. `challenge_spam(accused)` - Challenge a suspected spammer (forfeit bond)
6. `refund_bond()` - Refund a reserved bond after verification

### Events
- `ProfileRegistered` - User registered a public key
- `MessageHashStored` - Message hash recorded on-chain
- `ContactApproved` - Contact added to whitelist
- `ContactRemoved` - Contact removed from whitelist
- `BondReserved` - Spam protection bond reserved
- `BondForfeited` - Bond forfeited due to spam challenge
- `BondRefunded` - Bond refunded after verification

## Configuration Traits

The pallet requires the following runtime configuration:
- `RuntimeEvent` - Event type for this pallet
- `Time` - Time provider for timestamps
- `Currency` - Currency for spam bonds (must support reservable currency)
- `SpamBond` - Amount to reserve as spam protection
- `WeightInfo` - Weight information for dispatchables

## Example Runtime Configuration

```rust
impl pallet_messaging::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Time = Timestamp;
    type Currency = Balances;
    type SpamBond = ConstU128<1_000_000_000>; // 1 token
    type WeightInfo = ();
}
```

## Security Notes

- Off-chain message payloads should be encrypted independently
- Message hashes are immutable once stored
- Bonds are reserved (not yet transferred) until challenge or refund
- Runtime integrators should implement proper off-chain verification and call `refund_bond` after verification passes
