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
		let game_index: u32 = 0;
		#[extrinsic_call]
		create_game(RawOrigin::Signed(caller), bet);

		assert_eq!(Tictactoe::<T>::games(game_index).unwrap().bet, bet);
	}

	#[benchmark]
	fn join_game() {
		let host = account("host", 0, 0);
		T::Currency::make_free_balance_be(&host, BalanceOf::<T>::max_value());
		let _ = Tictactoe::<T>::create_game(
			RawOrigin::Signed(host.clone()).into(),
			T::Currency::minimum_balance(),
		);
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		#[extrinsic_call]
		join_game(RawOrigin::Signed(caller.clone()), 0u32);

		assert_eq!(Tictactoe::<T>::games(0).unwrap().payout_addresses.1, Some(caller));
	}

	#[benchmark]
	fn end_game() {
		T::Currency::make_free_balance_be(&Tictactoe::<T>::account_id(), 1000u32.into());

		let host = account("host", 0, 0);
		let bet = 1000u32.into();
		T::Currency::make_free_balance_be(&host, 10000000u32.into());
		let _ = Tictactoe::<T>::create_game(RawOrigin::Signed(host.clone()).into(), bet);

		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, 10000000u32.into());
		let _ = Tictactoe::<T>::join_game(RawOrigin::Signed(caller.clone()).into(), 0u32);

		assert!(Tictactoe::<T>::games(0).unwrap().bet == bet);
		assert_eq!(
			Tictactoe::<T>::games(0).unwrap().payout_addresses,
			(Some(host.clone()), Some(caller.clone()))
		);

		let _ =
			Tictactoe::<T>::end_game(RawOrigin::Signed(host.clone()).into(), 0u32, host.clone());
		#[extrinsic_call]
		end_game(RawOrigin::Signed(caller.clone()), 0u32, host.clone());

		assert_eq!(Tictactoe::<T>::games(0).unwrap().handshake, (Some(host.clone()), Some(host)));
	}

	#[benchmark]
	fn set_safeguard_deposit() {
		let deposit_value = 1000u32.into();
		#[extrinsic_call]
		set_safeguard_deposit(RawOrigin::Root, deposit_value);
		assert_eq!(Tictactoe::<T>::safeguard_deposit(), deposit_value);
	}

	#[benchmark]
	fn force_end_game() {
		T::Currency::make_free_balance_be(&Tictactoe::<T>::account_id(), 1000u32.into());

		let deposit_value = 1000u32.into();
		let _ = Tictactoe::<T>::set_safeguard_deposit(RawOrigin::Root.into(), deposit_value);

		let initial_balance = 10000000u32.into();
		let host = account("host", 0, 0);
		T::Currency::make_free_balance_be(&host, initial_balance);
		let joiner = account("joiner", 0, 0);
		T::Currency::make_free_balance_be(&joiner, initial_balance);

		let bet = 1000u32.into();
		let _ = Tictactoe::<T>::create_game(RawOrigin::Signed(host.clone()).into(), bet);
		let _ = Tictactoe::<T>::join_game(RawOrigin::Signed(joiner.clone()).into(), 0u32);

		let _ =
			Tictactoe::<T>::end_game(RawOrigin::Signed(host.clone()).into(), 0u32, host.clone());
		let _ = Tictactoe::<T>::end_game(
			RawOrigin::Signed(joiner.clone()).into(),
			0u32,
			joiner.clone(),
		);

		assert_eq!(Tictactoe::<T>::games(0).unwrap().handshake, (Some(host.clone()), Some(joiner)));

		#[extrinsic_call]
		force_end_game(RawOrigin::Root, 0u32, host.clone(), host.clone());

		assert_eq!(T::Currency::free_balance(&host), initial_balance.saturating_add(bet));
	}

	#[benchmark]
	fn withdraw_funds() {
		T::Currency::make_free_balance_be(&Tictactoe::<T>::account_id(), 100000u32.into());
		let beneficiary: T::AccountId = account("beneficiary", 0, 0);
		let amount = 1000u32.into();
		#[extrinsic_call]
		withdraw_funds(RawOrigin::Root, amount, beneficiary.clone());
		assert_eq!(T::Currency::free_balance(&beneficiary), amount);
	}

	impl_benchmark_test_suite!(Tictactoe, crate::mock::new_test_ext(), crate::mock::Test);
}
