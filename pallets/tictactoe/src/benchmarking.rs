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
	sp_runtime::traits::{Bounded, Zero},
	storage::bounded_vec::BoundedVec,
	traits::{EnsureOrigin, OnInitialize},
};
use frame_system::RawOrigin;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_game() {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let bet = T::Currency::minimum_balance();
		let game_index:u32 = 0;
		#[extrinsic_call]
		create_game(RawOrigin::Signed(caller), bet);

		assert_eq!(Tictactoe::<T>::games(game_index).unwrap().bet, bet);
	}


	//TBD How to achieve a state ? Create a game and then call join for measuring weight.

	impl_benchmark_test_suite!(Tictactoe, crate::mock::new_test_ext(), crate::mock::Test);
}
