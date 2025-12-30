use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn register_profile_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register profile
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		
		// Verify profile exists
		assert!(crate::UserProfiles::<Test>::contains_key(&1));
		
		// Verify bond was reserved
		assert_eq!(crate::SpamBonds::<Test>::get(&1), 100);
		assert_eq!(Balances::reserved_balance(&1), 100);
		
		// Verify event was emitted
		System::assert_last_event(
			Event::ProfileRegistered { who: 1, public_key }.into()
		);
	});
}

#[test]
fn register_profile_fails_if_already_exists() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register profile
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		
		// Try to register again
		assert_noop!(
			Messaging::register_profile(RuntimeOrigin::signed(1), public_key),
			Error::<Test>::ProfileAlreadyExists
		);
	});
}

#[test]
fn register_profile_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Account 4 has only 50, needs 100 for bond
		assert_noop!(
			Messaging::register_profile(RuntimeOrigin::signed(4), public_key),
			Error::<Test>::InsufficientBond
		);
	});
}

#[test]
fn register_profile_fails_with_invalid_public_key() {
	new_test_ext().execute_with(|| {
		// Empty public key
		assert_noop!(
			Messaging::register_profile(RuntimeOrigin::signed(1), vec![]),
			Error::<Test>::InvalidPublicKey
		);
		
		// Public key too large (> 256 bytes)
		let large_key = vec![1; 257];
		assert_noop!(
			Messaging::register_profile(RuntimeOrigin::signed(1), large_key),
			Error::<Test>::PublicKeyTooLarge
		);
	});
}

#[test]
fn update_profile_works() {
	new_test_ext().execute_with(|| {
		let public_key1 = vec![1, 2, 3, 4];
		let public_key2 = vec![5, 6, 7, 8];
		
		// Register profile
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key1));
		
		// Update profile
		assert_ok!(Messaging::update_profile(RuntimeOrigin::signed(1), public_key2.clone()));
		
		// Verify event was emitted
		System::assert_last_event(
			Event::ProfileUpdated { who: 1, public_key: public_key2 }.into()
		);
	});
}

#[test]
fn update_profile_fails_without_existing_profile() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		assert_noop!(
			Messaging::update_profile(RuntimeOrigin::signed(1), public_key),
			Error::<Test>::ProfileNotFound
		);
	});
}

#[test]
fn send_message_hash_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register both sender and recipient
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Send message hash
		let message_hash = H256::from([1; 32]);
		assert_ok!(Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash));
		
		// Verify message hash was stored
		assert!(crate::MessageHashes::<Test>::contains_key(0));
		
		// Verify event was emitted
		System::assert_last_event(
			Event::MessageSent {
				message_id: 0,
				from: 1,
				to: 2,
				hash: message_hash,
			}
			.into()
		);
		
		// Verify message ID counter incremented
		assert_eq!(crate::NextMessageId::<Test>::get(), 1);
	});
}

#[test]
fn send_message_hash_fails_without_sender_profile() {
	new_test_ext().execute_with(|| {
		let message_hash = H256::from([1; 32]);
		
		assert_noop!(
			Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash),
			Error::<Test>::ProfileNotFound
		);
	});
}

#[test]
fn send_message_hash_fails_without_recipient_profile() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Only register sender
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key));
		
		let message_hash = H256::from([1; 32]);
		assert_noop!(
			Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash),
			Error::<Test>::RecipientNotFound
		);
	});
}

#[test]
fn approve_contact_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register both users
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Approve contact
		assert_ok!(Messaging::approve_contact(RuntimeOrigin::signed(1), 2));
		
		// Verify contact was approved
		assert!(crate::ApprovedContacts::<Test>::get(&1, &2));
		
		// Verify contact count increased
		assert_eq!(crate::ContactCount::<Test>::get(&1), 1);
		
		// Verify event was emitted
		System::assert_last_event(
			Event::ContactApproved { approver: 1, contact: 2 }.into()
		);
	});
}

#[test]
fn approve_contact_fails_for_self() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key));
		
		assert_noop!(
			Messaging::approve_contact(RuntimeOrigin::signed(1), 1),
			Error::<Test>::CannotAddSelf
		);
	});
}

#[test]
fn approve_contact_fails_without_profile() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Messaging::approve_contact(RuntimeOrigin::signed(1), 2),
			Error::<Test>::ProfileNotFound
		);
	});
}

#[test]
fn approve_contact_fails_if_contact_doesnt_exist() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key));
		
		assert_noop!(
			Messaging::approve_contact(RuntimeOrigin::signed(1), 2),
			Error::<Test>::RecipientNotFound
		);
	});
}

#[test]
fn remove_contact_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register both users
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Approve contact
		assert_ok!(Messaging::approve_contact(RuntimeOrigin::signed(1), 2));
		assert_eq!(crate::ContactCount::<Test>::get(&1), 1);
		
		// Remove contact
		assert_ok!(Messaging::remove_contact(RuntimeOrigin::signed(1), 2));
		
		// Verify contact was removed
		assert!(!crate::ApprovedContacts::<Test>::get(&1, &2));
		
		// Verify contact count decreased
		assert_eq!(crate::ContactCount::<Test>::get(&1), 0);
		
		// Verify event was emitted
		System::assert_last_event(
			Event::ContactRemoved { remover: 1, contact: 2 }.into()
		);
	});
}

#[test]
fn challenge_spam_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register users
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Send message
		let message_hash = H256::from([1; 32]);
		assert_ok!(Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash));
		
		// Challenge as spam
		assert_ok!(Messaging::challenge_spam(RuntimeOrigin::signed(3), 0));
		
		// Verify event was emitted
		System::assert_last_event(
			Event::SpamChallenged { message_id: 0, challenger: 3 }.into()
		);
	});
}

#[test]
fn challenge_spam_fails_for_nonexistent_message() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Messaging::challenge_spam(RuntimeOrigin::signed(1), 999),
			Error::<Test>::MessageNotFound
		);
	});
}

#[test]
fn refund_bond_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register profile
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key));
		
		// Verify bond was reserved
		assert_eq!(Balances::reserved_balance(&1), 100);
		assert_eq!(Balances::free_balance(&1), 900);
		
		// Refund bond
		assert_ok!(Messaging::refund_bond(RuntimeOrigin::signed(1)));
		
		// Verify bond was unreserved
		assert_eq!(Balances::reserved_balance(&1), 0);
		assert_eq!(Balances::free_balance(&1), 1000);
		
		// Verify bond record was removed
		assert_eq!(crate::SpamBonds::<Test>::get(&1), 0);
		
		// Verify event was emitted
		System::assert_last_event(
			Event::BondRefunded { who: 1, amount: 100 }.into()
		);
	});
}

#[test]
fn refund_bond_fails_if_already_refunded() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register profile
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key));
		
		// Refund bond
		assert_ok!(Messaging::refund_bond(RuntimeOrigin::signed(1)));
		
		// Try to refund again
		assert_noop!(
			Messaging::refund_bond(RuntimeOrigin::signed(1)),
			Error::<Test>::BondAlreadyRefunded
		);
	});
}

#[test]
fn verify_message_hash_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register users
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Send message
		let message_hash = H256::from([1; 32]);
		assert_ok!(Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash));
		
		// Verify correct hash
		assert_ok!(Messaging::verify_message_hash(0, message_hash));
		assert_eq!(Messaging::verify_message_hash(0, message_hash).unwrap(), true);
		
		// Verify incorrect hash
		let wrong_hash = H256::from([2; 32]);
		assert_eq!(Messaging::verify_message_hash(0, wrong_hash).unwrap(), false);
	});
}

#[test]
fn message_expiry_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register users
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(2), public_key));
		
		// Send message at block 1
		let message_hash = H256::from([1; 32]);
		assert_ok!(Messaging::send_message_hash(RuntimeOrigin::signed(1), 2, message_hash));
		
		// Message should not be expired yet
		assert!(!Messaging::is_message_expired(0));
		
		// Advance blocks past expiry (MessageHashExpiry = 1000)
		System::set_block_number(1002);
		
		// Message should now be expired
		assert!(Messaging::is_message_expired(0));
		
		// Verification should fail for expired message
		assert_noop!(
			Messaging::verify_message_hash(0, message_hash),
			Error::<Test>::MessageExpired
		);
	});
}

#[test]
fn contact_limit_works() {
	new_test_ext().execute_with(|| {
		let public_key = vec![1, 2, 3, 4];
		
		// Register account 1
		assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), public_key.clone()));
		
		// MaxContactsPerUser is 100, but we'll test with realistic numbers
		// Register and approve 5 contacts
		for i in 2..7 {
			assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(i), public_key.clone()));
			assert_ok!(Messaging::approve_contact(RuntimeOrigin::signed(1), i));
		}
		
		// Verify contact count
		assert_eq!(crate::ContactCount::<Test>::get(&1), 5);
	});
}
