//! Weights for pallet-secure-messaging
#![allow(unused)]
use frame_support::weights::Weight;

pub trait WeightInfo {
    fn register_profile() -> Weight;
    fn send_message_hash() -> Weight;
    fn approve_contact() -> Weight;
    fn remove_contact() -> Weight;
    fn challenge_spam() -> Weight;
    fn refund_bond() -> Weight;
}

impl WeightInfo for () {
    fn register_profile() -> Weight { Weight::from_parts(10_000, 0) }
    fn send_message_hash() -> Weight { Weight::from_parts(50_000, 0) }
    fn approve_contact() -> Weight { Weight::from_parts(10_000, 0) }
    fn remove_contact() -> Weight { Weight::from_parts(10_000, 0) }
    fn challenge_spam() -> Weight { Weight::from_parts(20_000, 0) }
    fn refund_bond() -> Weight { Weight::from_parts(10_000, 0) }
}
