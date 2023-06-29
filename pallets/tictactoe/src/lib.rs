#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{
	sp_runtime::{traits::{AccountIdConversion,Zero}, DispatchError},
	traits::{Currency, Get,ExistenceRequirement::KeepAlive},
	PalletId, RuntimeDebug,
};

pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(
	Clone, Encode, Decode, Default, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
//TBD: If transfer of jackpot breaks due to existencial deposits, add fee param.
pub struct Game<Balance, AccountId> {
	bet: Option<Balance>,
	jackpot: Option<Balance>,
	payout_addresses: (AccountId, AccountId),
	ended: bool,
}

#[frame_support::pallet]
pub mod pallet {

	//TBD: For accessing imports outside of pallet mod
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Jackpot Pallet Id
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency trait for managing currency operations
		type Currency: Currency<Self::AccountId>;

		/// Event emission
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A game has been created.
		GameCreated { game_index: u32 },
		/// A player has won a game.
		GameWon { winner: T::AccountId, jackpot: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		IndexOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn game_index)]
	pub(crate) type GameIndex<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn games)]
	pub(crate) type Games<T: Config> =
		StorageMap<_, Twox64Concat, u32, Game<BalanceOf<T>, T::AccountId>, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T>{
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn start_game (origin: OriginFor<T>, bet: BalanceOf<T>) -> DispatchResult{
			let caller = ensure_signed(origin.clone())?;
			ensure!(!bet.is_zero(), "Bet must be greater than 0");
			T::Currency::transfer(&caller, &Self::account_id(), bet, KeepAlive)?;
			let game_index = Self::game_index();
			let game = Game {
				bet: Some(bet),
				jackpot: Some(bet),
				// TBD: How to set an "empty" account id?
				payout_addresses: (caller.clone(), caller.clone()),
				ended: false,
			};
			let new_game_index = game_index.checked_add(1).ok_or(Error::<T>::IndexOverflow)?;
			Games::<T>::insert(game_index, game);
			GameIndex::<T>::put(new_game_index);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// The account ID of the pallet which is the jackpot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

}
