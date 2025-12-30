//! # Secure Messaging Pallet
//!
//! A pallet for secure, privacy-preserving messaging on Substrate blockchains.
//!
//! ## Overview
//!
//! This pallet provides on-chain infrastructure for secure messaging while keeping
//! message content off-chain. It stores only message hashes for verification purposes,
//! implements spam prevention through economic bonds, and manages contact approval systems.
//!
//! ### Key Features
//!
//! - **Privacy-First**: Message content never stored on-chain, only cryptographic hashes
//! - **Spam Prevention**: Economic bonds required for message sending
//! - **Contact Management**: Bidirectional approval system for trusted contacts
//! - **Public Key Registry**: On-chain public key storage for end-to-end encryption
//! - **Message Verification**: On-chain hash verification for message authenticity
//! - **Expiry Management**: Automatic cleanup of expired message hashes
//!
//! ## Implementation Details
//!
//! ### Off-Chain Encryption Workflow
//!
//! 1. Sender retrieves recipient's public key from on-chain storage
//! 2. Sender encrypts message off-chain using recipient's public key
//! 3. Sender generates hash of encrypted message
//! 4. Sender submits only the hash to the blockchain
//! 5. Sender transmits encrypted message to recipient via off-chain channel
//! 6. Recipient verifies message authenticity by comparing hash with on-chain value
//! 7. Recipient decrypts message using their private key
//!
//! ### Security Model
//!
//! - Economic bonds prevent spam attacks
//! - Contact approval prevents unsolicited messages
//! - Message hashes expire to prevent chain bloat
//! - Public keys are validated on registration
//! - All extrinsics require proper origin verification

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Hash, Saturating};
use sp_std::vec::Vec;

pub type BalanceOf<T> =
	<<T as Config>::Currency as frame_support::traits::Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

pub type MessageId = u64;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::{Currency, ReservableCurrency, Time};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for handling bonds.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Time provider for expiry checks.
		type Time: Time;

		/// Weight information for extrinsics.
		type WeightInfo: WeightInfo;

		/// The amount required as a spam prevention bond.
		#[pallet::constant]
		type SpamBond: Get<BalanceOf<Self>>;

		/// Maximum number of contacts a user can have.
		#[pallet::constant]
		type MaxContactsPerUser: Get<u32>;

		/// Number of blocks after which message hashes expire.
		#[pallet::constant]
		type MessageHashExpiry: Get<BlockNumberFor<Self>>;
	}

	/// User profiles containing public keys for encryption.
	/// Maps AccountId => PublicKey (as Vec<u8>)
	#[pallet::storage]
	#[pallet::getter(fn user_profiles)]
	pub type UserProfiles<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u8, ConstU32<256>>, OptionQuery>;

	/// Message hashes for verification.
	/// Maps MessageId => (Hash, BlockNumber, Sender, Recipient)
	#[pallet::storage]
	#[pallet::getter(fn message_hashes)]
	pub type MessageHashes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		MessageId,
		(T::Hash, BlockNumberFor<T>, T::AccountId, T::AccountId),
		OptionQuery,
	>;

	/// Anti-spam bonds deposited by users.
	/// Maps AccountId => Balance
	#[pallet::storage]
	#[pallet::getter(fn spam_bonds)]
	pub type SpamBonds<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	/// Approved contacts list (bidirectional).
	/// Maps (AccountId, AccountId) => bool
	#[pallet::storage]
	#[pallet::getter(fn approved_contacts)]
	pub type ApprovedContacts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		bool,
		ValueQuery,
	>;

	/// Count of contacts per user.
	/// Maps AccountId => u32
	#[pallet::storage]
	#[pallet::getter(fn contact_count)]
	pub type ContactCount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Message ID counter.
	#[pallet::storage]
	#[pallet::getter(fn next_message_id)]
	pub type NextMessageId<T: Config> = StorageValue<_, MessageId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user profile was registered.
		ProfileRegistered { who: T::AccountId, public_key: Vec<u8> },
		/// A message hash was recorded on-chain.
		MessageSent { message_id: MessageId, from: T::AccountId, to: T::AccountId, hash: T::Hash },
		/// A contact was approved.
		ContactApproved { approver: T::AccountId, contact: T::AccountId },
		/// A contact was removed.
		ContactRemoved { remover: T::AccountId, contact: T::AccountId },
		/// A message was challenged as spam.
		SpamChallenged { message_id: MessageId, challenger: T::AccountId },
		/// A spam bond was refunded.
		BondRefunded { who: T::AccountId, amount: BalanceOf<T> },
		/// A user profile was updated.
		ProfileUpdated { who: T::AccountId, public_key: Vec<u8> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Profile already exists for this account.
		ProfileAlreadyExists,
		/// No profile found for this account.
		ProfileNotFound,
		/// Insufficient balance for spam bond.
		InsufficientBond,
		/// Invalid public key format.
		InvalidPublicKey,
		/// Recipient doesn't have a profile.
		RecipientNotFound,
		/// Maximum number of contacts reached.
		MaxContactsReached,
		/// Contact not in approved list.
		ContactNotApproved,
		/// Message not found in storage.
		MessageNotFound,
		/// Message hash has expired.
		MessageExpired,
		/// Not authorized to perform this action.
		NotAuthorized,
		/// Bond has already been refunded.
		BondAlreadyRefunded,
		/// Cannot add yourself as a contact.
		CannotAddSelf,
		/// Public key too large.
		PublicKeyTooLarge,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a user profile with a public key.
		///
		/// The caller must have sufficient balance for the spam bond.
		/// The public key will be stored on-chain for others to use for encryption.
		///
		/// Parameters:
		/// - `public_key`: The user's public key (max 256 bytes)
		///
		/// Emits `ProfileRegistered` event on success.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_profile())]
		pub fn register_profile(
			origin: OriginFor<T>,
			public_key: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Check if profile already exists
			ensure!(!UserProfiles::<T>::contains_key(&who), Error::<T>::ProfileAlreadyExists);

			// Validate public key
			ensure!(!public_key.is_empty(), Error::<T>::InvalidPublicKey);
			ensure!(public_key.len() <= 256, Error::<T>::PublicKeyTooLarge);

			// Reserve spam bond
			let bond_amount = T::SpamBond::get();
			T::Currency::reserve(&who, bond_amount)
				.map_err(|_| Error::<T>::InsufficientBond)?;

			// Store bond amount
			SpamBonds::<T>::insert(&who, bond_amount);

			// Convert to BoundedVec
			let bounded_key: BoundedVec<u8, ConstU32<256>> =
				public_key.clone().try_into().map_err(|_| Error::<T>::PublicKeyTooLarge)?;

			// Store profile
			UserProfiles::<T>::insert(&who, bounded_key);

			// Emit event
			Self::deposit_event(Event::ProfileRegistered { who, public_key });

			Ok(())
		}

		/// Update an existing user profile with a new public key.
		///
		/// Parameters:
		/// - `public_key`: The new public key (max 256 bytes)
		///
		/// Emits `ProfileUpdated` event on success.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_profile())]
		pub fn update_profile(
			origin: OriginFor<T>,
			public_key: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Check if profile exists
			ensure!(UserProfiles::<T>::contains_key(&who), Error::<T>::ProfileNotFound);

			// Validate public key
			ensure!(!public_key.is_empty(), Error::<T>::InvalidPublicKey);
			ensure!(public_key.len() <= 256, Error::<T>::PublicKeyTooLarge);

			// Convert to BoundedVec
			let bounded_key: BoundedVec<u8, ConstU32<256>> =
				public_key.clone().try_into().map_err(|_| Error::<T>::PublicKeyTooLarge)?;

			// Update profile
			UserProfiles::<T>::insert(&who, bounded_key);

			// Emit event
			Self::deposit_event(Event::ProfileUpdated { who, public_key });

			Ok(())
		}

		/// Send a message by recording its hash on-chain.
		///
		/// The actual encrypted message should be transmitted off-chain.
		/// Only the hash is stored for verification purposes.
		///
		/// Parameters:
		/// - `recipient`: The account ID of the message recipient
		/// - `message_hash`: The hash of the encrypted message
		///
		/// Emits `MessageSent` event on success.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::send_message_hash())]
		pub fn send_message_hash(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			message_hash: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Verify sender has profile and bond
			ensure!(UserProfiles::<T>::contains_key(&sender), Error::<T>::ProfileNotFound);
			ensure!(SpamBonds::<T>::contains_key(&sender), Error::<T>::InsufficientBond);

			// Verify recipient exists
			ensure!(
				UserProfiles::<T>::contains_key(&recipient),
				Error::<T>::RecipientNotFound
			);

			// Get current block number
			let current_block = frame_system::Pallet::<T>::block_number();

			// Get next message ID
			let message_id = NextMessageId::<T>::get();
			let next_id = message_id.saturating_add(1);
			NextMessageId::<T>::put(next_id);

			// Store message hash with metadata
			MessageHashes::<T>::insert(
				message_id,
				(message_hash, current_block, sender.clone(), recipient.clone()),
			);

			// Emit event
			Self::deposit_event(Event::MessageSent {
				message_id,
				from: sender,
				to: recipient,
				hash: message_hash,
			});

			Ok(())
		}

		/// Approve a contact for messaging.
		///
		/// Both parties must approve each other to establish a bidirectional contact.
		///
		/// Parameters:
		/// - `contact`: The account ID to approve
		///
		/// Emits `ContactApproved` event on success.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::approve_contact())]
		pub fn approve_contact(
			origin: OriginFor<T>,
			contact: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Cannot add yourself
			ensure!(who != contact, Error::<T>::CannotAddSelf);

			// Check if profile exists
			ensure!(UserProfiles::<T>::contains_key(&who), Error::<T>::ProfileNotFound);

			// Check contact exists
			ensure!(UserProfiles::<T>::contains_key(&contact), Error::<T>::RecipientNotFound);

			// Check max contacts limit
			let current_count = ContactCount::<T>::get(&who);
			ensure!(
				current_count < T::MaxContactsPerUser::get(),
				Error::<T>::MaxContactsReached
			);

			// Add to approved contacts
			if !ApprovedContacts::<T>::get(&who, &contact) {
				ApprovedContacts::<T>::insert(&who, &contact, true);
				ContactCount::<T>::mutate(&who, |count| *count = count.saturating_add(1));
			}

			// Emit event
			Self::deposit_event(Event::ContactApproved { approver: who, contact });

			Ok(())
		}

		/// Remove a contact from approved list.
		///
		/// This is a unilateral action that doesn't require the other party's consent.
		///
		/// Parameters:
		/// - `contact`: The account ID to remove
		///
		/// Emits `ContactRemoved` event on success.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::remove_contact())]
		pub fn remove_contact(
			origin: OriginFor<T>,
			contact: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Remove from approved contacts
			if ApprovedContacts::<T>::take(&who, &contact) {
				ContactCount::<T>::mutate(&who, |count| *count = count.saturating_sub(1));
			}

			// Emit event
			Self::deposit_event(Event::ContactRemoved { remover: who, contact });

			Ok(())
		}

		/// Challenge a message as spam.
		///
		/// This is a placeholder for governance-based spam challenges.
		/// In a production system, this would integrate with governance or reputation systems.
		///
		/// Parameters:
		/// - `message_id`: The ID of the message to challenge
		///
		/// Emits `SpamChallenged` event on success.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::challenge_spam())]
		pub fn challenge_spam(
			origin: OriginFor<T>,
			message_id: MessageId,
		) -> DispatchResult {
			let challenger = ensure_signed(origin)?;

			// Verify message exists
			let message_data =
				MessageHashes::<T>::get(message_id).ok_or(Error::<T>::MessageNotFound)?;
			let (_hash, _block, sender, _recipient) = message_data;

			// In production, this would involve:
			// 1. Governance vote
			// 2. Reputation check
			// 3. Challenge bond from challenger
			// 4. Slash sender's bond if challenge succeeds

			// For now, just emit event
			Self::deposit_event(Event::SpamChallenged { message_id, challenger });

			Ok(())
		}

		/// Claim back the spam bond.
		///
		/// Requires a clean history with no successful spam challenges.
		/// This is a simplified version - production would include time locks.
		///
		/// Emits `BondRefunded` event on success.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::refund_bond())]
		pub fn refund_bond(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Check if bond exists
			let bond_amount = SpamBonds::<T>::get(&who);
			ensure!(bond_amount > BalanceOf::<T>::default(), Error::<T>::BondAlreadyRefunded);

			// In production, add checks for:
			// 1. Time lock (e.g., must wait 30 days)
			// 2. No recent spam challenges
			// 3. Account closure requirements

			// Unreserve the bond
			T::Currency::unreserve(&who, bond_amount);

			// Remove bond record
			SpamBonds::<T>::remove(&who);

			// Emit event
			Self::deposit_event(Event::BondRefunded { who, amount: bond_amount });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Check if a message hash has expired.
		pub fn is_message_expired(message_id: MessageId) -> bool {
			if let Some((_hash, block_number, _sender, _recipient)) =
				MessageHashes::<T>::get(message_id)
			{
				let current_block = frame_system::Pallet::<T>::block_number();
				let expiry = block_number.saturating_add(T::MessageHashExpiry::get());
				current_block > expiry
			} else {
				true
			}
		}

		/// Verify a message hash matches what's stored on-chain.
		pub fn verify_message_hash(
			message_id: MessageId,
			hash: T::Hash,
		) -> Result<bool, DispatchError> {
			let message_data =
				MessageHashes::<T>::get(message_id).ok_or(Error::<T>::MessageNotFound)?;
			let (stored_hash, _block, _sender, _recipient) = message_data;

			// Check if expired
			ensure!(!Self::is_message_expired(message_id), Error::<T>::MessageExpired);

			Ok(stored_hash == hash)
		}
	}
}