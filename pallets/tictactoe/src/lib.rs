#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

///TBD: error: internal compiler error: encountered incremental compilation error with mir_built(76e5305fbe3bf3e0-1cbbbe6365e28f21)
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{
	sp_runtime::traits::{AccountIdConversion, Saturating, Zero},
	traits::{Currency, ExistenceRequirement::KeepAlive, Get},
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
		/// A player has joined a game.
		PlayerJoined { game_index: u32, player: T::AccountId },
		/// A game has ended.
		GameEnded { game_index: u32, winner: T::AccountId, jackpot: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The game does not exist.
		GameDoesNotExist,
		/// The gaame index has overflowed.
		IndexOverflow,
		/// The game has already ended.
		GameAlreadyEnded,
		/// The bet must be greater than 0.
		CantBeZero,
		/// The account is not a player of the game.
		NotAPlayer,
		/// Game is full.
		GameFull,
	}

	#[pallet::storage]
	#[pallet::getter(fn game_index)]
	pub(crate) type GameIndex<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn games)]
	pub(crate) type Games<T: Config> =
		StorageMap<_, Twox64Concat, u32, Game<BalanceOf<T>, T::AccountId>, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn start_game(origin: OriginFor<T>, bet: BalanceOf<T>) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			ensure!(!bet.is_zero(), Error::<T>::CantBeZero);
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
			Self::deposit_event(Event::GameCreated { game_index });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn join_game(origin: OriginFor<T>, game_index: u32) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			let game = Self::games(game_index).ok_or(Error::<T>::GameDoesNotExist)?;
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
			ensure!(game.payout_addresses.0 == game.payout_addresses.1, Error::<T>::GameFull);
			let bet = game.bet.ok_or(Error::<T>::GameDoesNotExist)?;
			T::Currency::transfer(&caller, &Self::account_id(), bet, KeepAlive)?;
			let host = game.payout_addresses.0;

			let new_jackpot = game.jackpot.ok_or(Error::<T>::GameDoesNotExist)?.saturating_add(bet);
			let new_game = Game {
				bet: game.bet,
				jackpot: Some(new_jackpot),
				payout_addresses: (host, caller.clone()),
				ended: game.ended,
			};
			Games::<T>::insert(game_index, new_game);
			Self::deposit_event(Event::PlayerJoined { game_index, player: caller });
			Ok(())
		}

		///TBD: How can we avoid someone to end a running game and steal funds to a prefered winner account?
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn end_game(
			origin: OriginFor<T>,
			game_index: u32,
			winner: T::AccountId,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let game = Self::games(game_index).ok_or(Error::<T>::GameDoesNotExist)?;
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
			let payout_addresses = game.payout_addresses;
			let host = payout_addresses.0;
			let joiner = payout_addresses.1;
			ensure!(winner == host || winner == joiner, Error::<T>::NotAPlayer);
			let jackpot = game.jackpot.ok_or(Error::<T>::GameDoesNotExist)?;
			let new_game = Game {
				bet: game.bet,
				jackpot: Some(Zero::zero()),
				payout_addresses: (host, joiner),
				ended: true,
			};
			Games::<T>::insert(game_index, new_game);
			let _ = T::Currency::transfer(&Self::account_id(), &winner, jackpot, KeepAlive)?;
			Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });
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
