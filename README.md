# Secure Messaging Pallet - Complete Documentation

## Overview

The Secure Messaging Pallet provides a privacy-preserving messaging infrastructure for Substrate-based blockchains. It implements on-chain hash verification while keeping message content entirely off-chain, ensuring both privacy and authenticity.

## Architecture

### Design Philosophy

1. **Privacy First**: Only message hashes are stored on-chain, never content
2. **Economic Security**: Spam prevention through required bonds
3. **User Control**: Bidirectional contact approval system
4. **Scalability**: Message hash expiry prevents blockchain bloat
5. **Flexibility**: Public key registry enables end-to-end encryption

## Features

### 1. User Profile Management

Users register their public encryption keys on-chain:

```rust
// Register profile with public key
Messaging::register_profile(origin, public_key: Vec<u8>)

// Update existing profile
Messaging::update_profile(origin, new_public_key: Vec<u8>)
```

**Key Points:**
- Public keys are stored in BoundedVec (max 256 bytes)
- Registration requires depositing a spam bond (configurable, default: 10 UNIT)
- Bonds are reserved, not transferred
- Users can update their keys anytime

### 2. Message Hash Recording

Send messages by recording cryptographic hashes:

```rust
Messaging::send_message_hash(
    origin,
    recipient: AccountId,
    message_hash: Hash
)
```

**Process:**
1. Sender must have registered profile and deposited bond
2. Recipient must have registered profile
3. Hash is stored with: (hash, block_number, sender, recipient)
4. Message ID is auto-incremented
5. Actual encrypted message transmitted off-chain

### 3. Contact Management

Bidirectional contact approval system:

```rust
// Approve a contact (one-way)
Messaging::approve_contact(origin, contact: AccountId)

// Remove a contact
Messaging::remove_contact(origin, contact: AccountId)
```

**Features:**
- Each user can approve up to `MaxContactsPerUser` contacts (default: 1000)
- Approval is unilateral - both parties must approve for bidirectional trust
- Removal is instant and one-sided
- Contact counts are tracked per user

### 4. Spam Prevention

Economic and governance-based spam prevention:

```rust
// Challenge a message as spam
Messaging::challenge_spam(origin, message_id: MessageId)

// Reclaim spam bond
Messaging::refund_bond(origin)
```

**Spam Prevention Mechanisms:**
- **Economic Bond**: Required deposit for profile registration
- **Challenge System**: Messages can be flagged as spam
- **Bond Slashing**: (Future) Successful challenges slash sender's bond
- **Reputation**: (Future) Integration with reputation systems

### 5. Message Verification

Helper functions for off-chain verification:

```rust
// Check if message has expired
Messaging::is_message_expired(message_id: MessageId) -> bool

// Verify message hash matches on-chain record
Messaging::verify_message_hash(message_id: MessageId, hash: Hash) -> Result<bool>
```

## Off-Chain Encryption Workflow

### Complete Message Flow

```
┌─────────┐                                          ┌───────────┐
│ Sender  │                                          │ Recipient │
└────┬────┘                                          └─────┬─────┘
     │                                                      │
     │ 1. Fetch recipient's public key from chain          │
     │─────────────────────────────────────────────────────▶
     │                                                      │
     │ 2. Encrypt message locally (off-chain)              │
     │    using recipient's public key                     │
     │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
     │                                                      │
     │ 3. Hash encrypted message                           │
     │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
     │                                                      │
     │ 4. Submit hash to blockchain                        │
     │─────────────────────────────────────────────────────▶
     │                    ┌──────────┐                     │
     │────────────────────▶Blockchain│◀─────────────────────
     │                    └──────────┘                     │
     │ 5. Transmit encrypted message off-chain             │
     │    (via P2P, IPFS, or direct connection)            │
     │─────────────────────────────────────────────────────▶
     │                                                      │
     │                                    6. Fetch hash    │
     │                                       from chain    │
     │                    ┌──────────┐                     │
     │                    │Blockchain│◀─────────────────────
     │                    └──────────┘                     │
     │                                                      │
     │                                    7. Verify hash   │
     │                                       matches       │
     │                    ◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│
     │                                                      │
     │                                    8. Decrypt using │
     │                                       private key   │
     │                    ◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│
     │                                                      │
```

### Implementation Example

#### Client-Side Implementation (Pseudocode)

```rust
// 1. SENDER: Encrypt and Send Message
async fn send_secure_message(
    sender_keypair: Keypair,
    recipient_account: AccountId,
    message: String,
) -> Result<MessageId> {
    // Fetch recipient's public key from chain
    let recipient_pubkey = query_chain_storage(
        "Messaging",
        "UserProfiles",
        recipient_account
    ).await?;
    
    // Encrypt message using recipient's public key (use libsodium, age, or similar)
    let encrypted_message = encrypt(
        message.as_bytes(),
        &recipient_pubkey
    )?;
    
    // Hash the encrypted message
    let message_hash = blake2_256(&encrypted_message);
    
    // Submit hash to blockchain
    let extrinsic = compose_extrinsic(
        "Messaging",
        "send_message_hash",
        (recipient_account, message_hash)
    );
    
    let message_id = submit_extrinsic(
        sender_keypair,
        extrinsic
    ).await?;
    
    // Transmit encrypted message off-chain (P2P, IPFS, etc.)
    transmit_off_chain(
        recipient_account,
        encrypted_message,
        message_id
    ).await?;
    
    Ok(message_id)
}

// 2. RECIPIENT: Receive and Verify Message
async fn receive_secure_message(
    recipient_keypair: Keypair,
    message_id: MessageId,
    encrypted_message: Vec<u8>,
) -> Result<String> {
    // Fetch message hash from blockchain
    let on_chain_data = query_chain_storage(
        "Messaging",
        "MessageHashes",
        message_id
    ).await?;
    
    let (stored_hash, block_num, sender, recipient) = on_chain_data;
    
    // Verify recipient matches
    ensure!(recipient == recipient_keypair.public(), "Not recipient");
    
    // Check message hasn't expired
    let current_block = get_current_block().await?;
    ensure!(
        current_block <= block_num + MESSAGE_EXPIRY,
        "Message expired"
    );
    
    // Verify hash matches
    let computed_hash = blake2_256(&encrypted_message);
    ensure!(computed_hash == stored_hash, "Hash mismatch - message tampered!");
    
    // Decrypt message using recipient's private key
    let decrypted_message = decrypt(
        &encrypted_message,
        &recipient_keypair.secret
    )?;
    
    Ok(String::from_utf8(decrypted_message)?)
}
```

#### Recommended Encryption Libraries

1. **Rust**: 
   - `sodiumoxide` (libsodium bindings)
   - `age` (modern encryption tool)
   - `orion` (pure Rust)

2. **JavaScript/TypeScript**:
   - `libsodium-wrappers`
   - `@noble/ciphers`
   - `tweetnacl`

3. **Python**:
   - `PyNaCl`
   - `cryptography`

## Storage Items

### UserProfiles
```rust
StorageMap<Blake2_128Concat, AccountId, BoundedVec<u8, ConstU32<256>>>
```
- **Key**: Account ID
- **Value**: Public encryption key (max 256 bytes)
- **Purpose**: Registry of user public keys for encryption

### MessageHashes
```rust
StorageMap<Blake2_128Concat, MessageId, (Hash, BlockNumber, AccountId, AccountId)>
```
- **Key**: Message ID
- **Value**: (Hash, Creation Block, Sender, Recipient)
- **Purpose**: Verification data for messages

### SpamBonds
```rust
StorageMap<Blake2_128Concat, AccountId, Balance>
```
- **Key**: Account ID
- **Value**: Reserved bond amount
- **Purpose**: Track spam prevention bonds

### ApprovedContacts
```rust
StorageDoubleMap<Blake2_128Concat, AccountId, Blake2_128Concat, AccountId, bool>
```
- **Keys**: (Approver Account, Contact Account)
- **Value**: Approval status
- **Purpose**: Bidirectional contact approval

### ContactCount
```rust
StorageMap<Blake2_128Concat, AccountId, u32>
```
- **Key**: Account ID
- **Value**: Number of approved contacts
- **Purpose**: Enforce max contacts limit

### NextMessageId
```rust
StorageValue<u64>
```
- **Value**: Next available message ID
- **Purpose**: Auto-incrementing message counter

## Events

```rust
ProfileRegistered { who: AccountId, public_key: Vec<u8> }
ProfileUpdated { who: AccountId, public_key: Vec<u8> }
MessageSent { message_id: MessageId, from: AccountId, to: AccountId, hash: Hash }
ContactApproved { approver: AccountId, contact: AccountId }
ContactRemoved { remover: AccountId, contact: AccountId }
SpamChallenged { message_id: MessageId, challenger: AccountId }
BondRefunded { who: AccountId, amount: Balance }
```

## Configuration Parameters

### Runtime Configuration

```rust
parameter_types! {
    // Spam bond: 10 UNIT (10,000,000,000,000)
    pub const SpamBond: Balance = 10 * UNIT;
    
    // Max contacts: 1000 per user
    pub const MaxContactsPerUser: u32 = 1000;
    
    // Hash expiry: 7 days (100,800 blocks at 6s/block)
    pub const MessageHashExpiry: BlockNumber = 7 * DAYS;
}
```

### Customization Options

Adjust these based on your chain's requirements:

1. **SpamBond**: Higher = more spam resistance, lower = more accessible
2. **MaxContactsPerUser**: Balance between usability and storage
3. **MessageHashExpiry**: Longer = better reliability, shorter = less storage

## Security Considerations

### Threat Model

1. **Spam Attacks**: Prevented by economic bonds
2. **Message Tampering**: Detected via hash verification
3. **Replay Attacks**: Prevented by unique message IDs and timestamps
4. **Storage Bloat**: Mitigated by message expiry
5. **Unsolicited Messages**: Prevented by contact approval (optional enforcement)

### Best Practices

1. **Key Management**:
   - Store private keys securely (hardware wallets recommended)
   - Rotate keys periodically using `update_profile`
   - Never share private keys

2. **Message Security**:
   - Always verify hash before decrypting
   - Check message hasn't expired
   - Validate sender is expected contact

3. **Bond Management**:
   - Only refund bonds from accounts with good history
   - Implement time locks in production (not included in basic version)
   - Monitor spam challenges

4. **Off-Chain Transmission**:
   - Use secure channels (TLS, Tor, etc.)
   - Consider IPFS for persistence
   - Implement retry logic for failed transmissions

## Testing

### Running Tests

```bash
# Run all tests
cargo test --package pallet-messaging

# Run specific test
cargo test --package pallet-messaging register_profile_works

# Run with output
cargo test --package pallet-messaging -- --nocapture
```

### Test Coverage

Current test suite covers:
- ✅ Profile registration
- ✅ Profile updates
- ✅ Message hash sending
- ✅ Contact approval/removal
- ✅ Spam challenges
- ✅ Bond refunds
- ✅ Hash verification
- ✅ Message expiry
- ✅ Error conditions
- ✅ Boundary conditions

## Benchmarking

### Running Benchmarks

```bash
# Generate weights
cargo build --release --features runtime-benchmarks
./target/release/solochain-template-node benchmark pallet \
    --chain dev \
    --pallet pallet_messaging \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output pallets/messaging/src/weights.rs
```

### Expected Weights

Approximate computational costs:
- `register_profile`: ~50M weight units
- `update_profile`: ~30M weight units
- `send_message_hash`: ~40M weight units
- `approve_contact`: ~35M weight units
- `remove_contact`: ~25M weight units
- `challenge_spam`: ~20M weight units
- `refund_bond`: ~30M weight units

## Integration Guide

### 1. Add to Workspace

Already done in the provided `Cargo.toml` files.

### 2. Configure Runtime

Already configured in `runtime/src/configs/mod.rs`.

### 3. Add to Runtime Construction

Already added at pallet index 7 in `runtime/src/lib.rs`.

### 4. Update Benchmarks

Already updated in `runtime/src/benchmarks.rs`.

### 5. Build and Run

```bash
# Build the node
cargo build --release

# Run in development mode
./target/release/solochain-template-node --dev

# Purge old chain data if needed
./target/release/solochain-template-node purge-chain --dev
```

## Frontend Integration

### Using Polkadot.js

```javascript
import { ApiPromise, WsProvider } from '@polkadot/api';

// Connect to node
const wsProvider = new WsProvider('ws://127.0.0.1:9944');
const api = await ApiPromise.create({ provider: wsProvider });

// Register profile
const publicKey = new Uint8Array([1, 2, 3, 4]); // Your public key
const tx = api.tx.messaging.registerProfile(publicKey);
await tx.signAndSend(sender);

// Send message hash
const recipient = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const messageHash = '0x1234...'; // Blake2-256 hash
const tx = api.tx.messaging.sendMessageHash(recipient, messageHash);
await tx.signAndSend(sender);

// Query profile
const profile = await api.query.messaging.userProfiles(accountId);
console.log('Public Key:', profile.toHex());

// Query message hash
const messageData = await api.query.messaging.messageHashes(0);
const [hash, blockNum, sender, recipient] = messageData.unwrap();
```

## Future Enhancements

### Planned Features

1. **Governance Integration**:
   - Council-based spam challenge resolution
   - Democratic bond slashing
   - Parameter adjustments via governance

2. **Advanced Spam Prevention**:
   - Reputation-based bond requirements
   - Progressive penalties for repeated spam
   - Whitelist/blacklist functionality

3. **Message Types**:
   - Support for different message formats
   - Metadata attachments
   - Group messaging support

4. **Enhanced Privacy**:
   - Zero-knowledge proofs for sender anonymity
   - Ring signatures
   - Stealth addresses

5. **Scalability**:
   - Off-chain workers for automated cleanup
   - Pagination for large contact lists
   - Archive old messages to separate storage

## Troubleshooting

### Common Issues

1. **InsufficientBond Error**:
   - Ensure account has enough free balance
   - Check bond amount: `SpamBond` parameter

2. **ProfileNotFound Error**:
   - Register profile first before messaging
   - Verify account address is correct

3. **MessageExpired Error**:
   - Message hash has passed expiry window
   - Resend the message with new hash

4. **PublicKeyTooLarge Error**:
   - Public keys must be ≤256 bytes
   - Use compressed key formats

## License

MIT-0 - See LICENSE file for details.

## Support

For issues, questions, or contributions:
- GitHub: [Your Repository]
- Discord: [Your Community]
- Email: [Your Email]

## Credits

Built with Substrate FRAME v2 by [Your Name/Team]
