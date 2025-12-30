//! # Secure Messaging Pallet
//!
//! Provides an on-chain registry for public keys and a signaling mechanism for off-chain
//! encrypted messages. Only message hashes are stored on-chain for verification and non-repudiation.
//!
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

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::traits::{Time, Currency, ReservableCurrency};
    use frame_support::traits::Get;
    use frame_support::BoundedVec;
    use scale_info::prelude::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Time provider for timestamps
        type Time: Time;
        /// Currency used to collect spam bonds
        type Currency: ReservableCurrency<Self::AccountId>;
        /// Amount reserved for spam protection
        type SpamBond: Get<<<Self as Config>::Currency as Currency<Self::AccountId>>::Balance>;
        /// Weight information
        type WeightInfo: WeightInfo;
    }

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Map an account to its public key (stored as bounded bytes). Public key bytes should be handled
    /// by off-chain code and are used for verifying identities.
    #[pallet::storage]
    #[pallet::getter(fn profiles)]
    pub type Profiles<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u8, ConstU32<512>>, OptionQuery>;

    /// Map a message identifier (client-provided) to the hash of the encrypted payload.
    #[pallet::storage]
    #[pallet::getter(fn message_hashes)]
    pub type MessageHashes<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, T::Hash, OptionQuery>;

    /// Bonds put up by accounts to prevent spam. Pallet records the reserved amount for bookkeeping.
    #[pallet::storage]
    #[pallet::getter(fn bonds)]
    pub type Bonds<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// Contacts (approved): (owner, contact) -> bool (exists)
    #[pallet::storage]
    #[pallet::getter(fn contacts)]
    pub type Contacts<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::AccountId, (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProfileRegistered { who: T::AccountId },
        MessageHashStored { who: T::AccountId, msg_id: T::Hash, msg_hash: T::Hash },
        ContactApproved { who: T::AccountId, contact: T::AccountId },
        ContactRemoved { who: T::AccountId, contact: T::AccountId },
        BondReserved { who: T::AccountId, amount: BalanceOf<T> },
        BondForfeited { who: T::AccountId, amount: BalanceOf<T>, challenger: T::AccountId },
        BondRefunded { who: T::AccountId, amount: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotRegistered,
        MessageAlreadyExists,
        ContactNotFound,
        InsufficientBond,
        NothingToForfeit,
        PublicKeyTooLarge,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(_block_number: BlockNumberFor<T>) {
            // Offchain worker hooks can be implemented by runtime integrators as needed.
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a profile public key for the caller.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_profile())]
        pub fn register_profile(origin: OriginFor<T>, public_key: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bounded_key: BoundedVec<u8, ConstU32<512>> = public_key.try_into()
                .map_err(|_| Error::<T>::PublicKeyTooLarge)?;
            Profiles::<T>::insert(&who, bounded_key);
            Self::deposit_event(Event::ProfileRegistered { who });
            Ok(())
        }

        /// Store a message hash on-chain as a verification/signaling mechanism. Caller must be registered.
        /// A small spam bond is reserved when sending (see `SpamBond`).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::send_message_hash())]
        pub fn send_message_hash(origin: OriginFor<T>, msg_id: T::Hash, msg_hash: T::Hash) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Profiles::<T>::contains_key(&who), Error::<T>::NotRegistered);
            ensure!(!MessageHashes::<T>::contains_key(&msg_id), Error::<T>::MessageAlreadyExists);

            // Reserve spam bond. If reserving fails, return error.
            let bond = T::SpamBond::get();
            let reserve_result = T::Currency::reserve(&who, bond);
            ensure!(reserve_result.is_ok(), Error::<T>::InsufficientBond);

            Bonds::<T>::insert(&who, bond);
            MessageHashes::<T>::insert(&msg_id, &msg_hash);

            Self::deposit_event(Event::BondReserved { who: who.clone(), amount: bond });
            Self::deposit_event(Event::MessageHashStored { who, msg_id, msg_hash });
            Ok(())
        }

        /// Approve a contact so they can message you without additional acceptance.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::approve_contact())]
        pub fn approve_contact(origin: OriginFor<T>, contact: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Contacts::<T>::insert(&who, &contact, ());
            Self::deposit_event(Event::ContactApproved { who, contact });
            Ok(())
        }

        /// Remove an approved contact.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::remove_contact())]
        pub fn remove_contact(origin: OriginFor<T>, contact: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Contacts::<T>::contains_key(&who, &contact), Error::<T>::ContactNotFound);
            Contacts::<T>::remove(&who, &contact);
            Self::deposit_event(Event::ContactRemoved { who, contact });
            Ok(())
        }

        /// Challenge a suspected spammer. If successful, the bond is forfeited to challenger.
        /// NOTE: This is a simple on-chain signalling action; integrators should expand dispute
        /// resolution and verification off-chain and call `refund_bond` when appropriate.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::challenge_spam())]
        pub fn challenge_spam(origin: OriginFor<T>, accused: T::AccountId) -> DispatchResult {
            let challenger = ensure_signed(origin)?;

            let bond = Bonds::<T>::get(&accused);
            ensure!(bond > BalanceOf::<T>::default(), Error::<T>::NothingToForfeit);

            // For simplicity, mark the bond as forfeited and emit event. Runtime integrator
            // may choose to actually slash/unreserve and transfer funds here.
            Bonds::<T>::remove(&accused);
            Self::deposit_event(Event::BondForfeited { who: accused, amount: bond, challenger });
            Ok(())
        }

        /// Refund a bond for an account (to be called after off-chain verification passes).
        /// Only callable by the account itself.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::refund_bond())]
        pub fn refund_bond(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bond = Bonds::<T>::get(&who);
            ensure!(bond > BalanceOf::<T>::default(), Error::<T>::NothingToForfeit);

            Bonds::<T>::remove(&who);
            // Real refund would unreserve the reserved balance. Integrators should implement
            // the precise money-movement semantics depending on their runtime.
            Self::deposit_event(Event::BondRefunded { who, amount: bond });
            Ok(())
        }
    }
}
