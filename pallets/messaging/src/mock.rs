use crate as pallet_messaging;
use frame_support::derive_impl;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(RuntimeCall, RuntimeEvent, RuntimeError, RuntimeOrigin)]
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system::Pallet<Test>;

    #[runtime::pallet_index(1)]
    pub type Messaging = pallet_messaging::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

// Simple mock for Time trait
pub struct MockTime;
impl frame_support::traits::Time for MockTime {
    type Moment = u64;
    fn now() -> Self::Moment {
        0
    }
}

impl pallet_messaging::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Time = MockTime;
    type Currency = ();
    type SpamBond = frame_support::traits::ConstU128<0>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
