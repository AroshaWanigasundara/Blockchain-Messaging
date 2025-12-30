use crate::{mock::*, Error, Event, Profiles, MessageHashes, Bonds};
use frame_support::{assert_noop, assert_ok};

#[test]
fn register_profile_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Messaging::register_profile(RuntimeOrigin::signed(1), b"pubkey".to_vec()));
        System::assert_last_event(Event::ProfileRegistered { who: 1 }.into());
    });
}

#[test]
fn send_message_hash_requires_registration() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let msg_id = <Test as frame_system::Config>::Hashing::hash_of(&b"id".to_vec());
        let msg_hash = <Test as frame_system::Config>::Hashing::hash_of(&b"payload".to_vec());
        assert_noop!(Messaging::send_message_hash(RuntimeOrigin::signed(1), msg_id, msg_hash), Error::<Test>::NotRegistered);
    });
}
