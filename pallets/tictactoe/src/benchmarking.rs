//! Benchmarking setup for pallet-template
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Tictactoe;
use frame_benchmarking::{
	impl_benchmark_test_suite,
	v1::{account, whitelisted_caller, BenchmarkError},
	v2::*,
};
use frame_support::{
	storage::bounded_vec::BoundedVec,
	traits::{EnsureOrigin, OnInitialize},
    sp_runtime::traits::{Bounded, Zero}
};
use frame_system::RawOrigin;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[benchmarks]
mod benchmarks {
	use super::*;
	//n: Linear<0, 100>

    #[benchmark]
    fn create_game() {
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let bet = T::Currency::minimum_balance();
        #[extrinsic_call]
        start_game(RawOrigin::Signed(caller), bet);
    }

	impl_benchmark_test_suite!(Tictactoe, crate::mock::new_test_ext(), crate::mock::Test);
}
