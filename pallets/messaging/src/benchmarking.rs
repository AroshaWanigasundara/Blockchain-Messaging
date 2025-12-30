//! Benchmarking setup for pallet-messaging

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Messaging;
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_std::vec;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_profile() {
		let caller: T::AccountId = whitelisted_caller();
		let public_key = vec![1u8; 32];
		
		// Fund the caller
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());

		#[extrinsic_call]
		register_profile(RawOrigin::Signed(caller.clone()), public_key);

		assert!(UserProfiles::<T>::contains_key(&caller));
	}

	#[benchmark]
	fn update_profile() {
		let caller: T::AccountId = whitelisted_caller();
		let public_key1 = vec![1u8; 32];
		let public_key2 = vec![2u8; 32];
		
		// Setup: register profile first
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(caller.clone()).into(), public_key1);

		#[extrinsic_call]
		update_profile(RawOrigin::Signed(caller.clone()), public_key2);

		assert!(UserProfiles::<T>::contains_key(&caller));
	}

	#[benchmark]
	fn send_message_hash() {
		let caller: T::AccountId = whitelisted_caller();
		let recipient: T::AccountId = account("recipient", 0, 0);
		let public_key = vec![1u8; 32];
		let message_hash = T::Hashing::hash_of(&[1u8; 32]);
		
		// Setup: register both users
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());
		let _ = T::Currency::make_free_balance_be(&recipient, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(caller.clone()).into(), public_key.clone());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(recipient.clone()).into(), public_key);

		#[extrinsic_call]
		send_message_hash(RawOrigin::Signed(caller), recipient, message_hash);

		assert_eq!(NextMessageId::<T>::get(), 1);
	}

	#[benchmark]
	fn approve_contact() {
		let caller: T::AccountId = whitelisted_caller();
		let contact: T::AccountId = account("contact", 0, 0);
		let public_key = vec![1u8; 32];
		
		// Setup: register both users
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());
		let _ = T::Currency::make_free_balance_be(&contact, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(caller.clone()).into(), public_key.clone());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(contact.clone()).into(), public_key);

		#[extrinsic_call]
		approve_contact(RawOrigin::Signed(caller.clone()), contact.clone());

		assert!(ApprovedContacts::<T>::get(&caller, &contact));
	}

	#[benchmark]
	fn remove_contact() {
		let caller: T::AccountId = whitelisted_caller();
		let contact: T::AccountId = account("contact", 0, 0);
		let public_key = vec![1u8; 32];
		
		// Setup: register both users and approve contact
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());
		let _ = T::Currency::make_free_balance_be(&contact, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(caller.clone()).into(), public_key.clone());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(contact.clone()).into(), public_key);
		let _ = Messaging::<T>::approve_contact(RawOrigin::Signed(caller.clone()).into(), contact.clone());

		#[extrinsic_call]
		remove_contact(RawOrigin::Signed(caller.clone()), contact.clone());

		assert!(!ApprovedContacts::<T>::get(&caller, &contact));
	}

	#[benchmark]
	fn challenge_spam() {
		let sender: T::AccountId = account("sender", 0, 0);
		let recipient: T::AccountId = account("recipient", 0, 0);
		let challenger: T::AccountId = whitelisted_caller();
		let public_key = vec![1u8; 32];
		let message_hash = T::Hashing::hash_of(&[1u8; 32]);
		
		// Setup: register users and send message
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&sender, bond_amount * 10u32.into());
		let _ = T::Currency::make_free_balance_be(&recipient, bond_amount * 10u32.into());
		let _ = T::Currency::make_free_balance_be(&challenger, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(sender.clone()).into(), public_key.clone());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(recipient.clone()).into(), public_key);
		let _ = Messaging::<T>::send_message_hash(RawOrigin::Signed(sender).into(), recipient, message_hash);
		
		let message_id = 0;

		#[extrinsic_call]
		challenge_spam(RawOrigin::Signed(challenger), message_id);
	}

	#[benchmark]
	fn refund_bond() {
		let caller: T::AccountId = whitelisted_caller();
		let public_key = vec![1u8; 32];
		
		// Setup: register profile
		let bond_amount = T::SpamBond::get();
		let _ = T::Currency::make_free_balance_be(&caller, bond_amount * 10u32.into());
		let _ = Messaging::<T>::register_profile(RawOrigin::Signed(caller.clone()).into(), public_key);

		#[extrinsic_call]
		refund_bond(RawOrigin::Signed(caller.clone()));

		assert_eq!(SpamBonds::<T>::get(&caller), BalanceOf::<T>::default());
	}

	impl_benchmark_test_suite!(Messaging, crate::mock::new_test_ext(), crate::mock::Test);
}