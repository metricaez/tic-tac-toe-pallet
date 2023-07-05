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

/// TBD: difference of using sp_runtime from frame_support or from sp_runtime directly
use frame_support::{
	sp_runtime::{
		traits::{AccountIdConversion, Saturating, Zero},
		DispatchError,
	},
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
	bet: Balance,
	payout_addresses: (Option<AccountId>, Option<AccountId>),
	ended: bool,
	handshake: (Option<AccountId>, Option<AccountId>),
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
		/// Bad address stored.
		BadAddress,
		/// Handshale already set
		HandshakeAlreadySet,
	}

	#[pallet::storage]
	#[pallet::getter(fn game_index)]
	pub(crate) type GameIndex<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn safeguard_deposit)]
	pub(crate) type SafeguardDeposit<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
			let transfer_amount = bet.saturating_add(Self::safeguard_deposit());
			T::Currency::transfer(&caller, &Self::account_id(), transfer_amount, KeepAlive)?;
			let game_index = Self::game_index();
			let game = Game {
				bet,
				payout_addresses: (Some(caller.clone()), None),
				ended: false,
				handshake: (None, None),
			};
			let new_game_index =
				game_index.checked_add(1).ok_or_else(|| Error::<T>::IndexOverflow)?;
			Games::<T>::insert(game_index, game);
			GameIndex::<T>::put(new_game_index);
			Self::deposit_event(Event::GameCreated { game_index });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn join_game(origin: OriginFor<T>, game_index: u32) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			let game = match Self::games(game_index) {
				Some(game) => game,
				None => return Err(Error::<T>::GameDoesNotExist.into()),
			};
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
			ensure!(game.payout_addresses.1 == None, Error::<T>::GameFull);
			let bet = game.bet;
			let transfer_amount = bet.saturating_add(Self::safeguard_deposit());
			T::Currency::transfer(&caller, &Self::account_id(), transfer_amount, KeepAlive)?;
			let host = game.payout_addresses.0;
			let new_game = Game {
				bet: game.bet,
				payout_addresses: (host, Some(caller.clone())),
				ended: game.ended,
				handshake: (None, None),
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

			let caller = ensure_signed(origin)?;
			let game = match Self::games(game_index) {
				Some(game) => game,
				None => return Err(Error::<T>::GameDoesNotExist.into()),
			};
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);

			let payout_addresses = game.payout_addresses;
			let host = payout_addresses.0.ok_or_else(|| Error::<T>::BadAddress)?;
			let joiner = payout_addresses.1.ok_or_else(|| Error::<T>::BadAddress)?;
			ensure!(caller == host || caller == joiner, Error::<T>::NotAPlayer);
			ensure!(winner == host || winner == joiner, Error::<T>::NotAPlayer);

			let new_game = Game {
				bet: game.bet,
				payout_addresses: (Some(host), Some(joiner)),
				ended: true,
				handshake: (None, None),
			};
			Games::<T>::insert(game_index, new_game);

			let jackpot = game.bet.saturating_mul(2u32.into());
			let _ = T::Currency::transfer(&Self::account_id(), &winner, jackpot, KeepAlive)?;
			
			Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn set_safeguard_deposit(
			origin: OriginFor<T>,
			deposit: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			SafeguardDeposit::<T>::put(deposit);
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

	fn update_handshake(
		caller: T::AccountId,
		host: T::AccountId,
		winner: T::AccountId,
		handshake: (Option<T::AccountId>, Option<T::AccountId>),
	) -> Result<(Option<T::AccountId>, Option<T::AccountId>), DispatchError> {
		let mut new_handshake = handshake;
		if caller == host {
			if new_handshake.0 == None {
				new_handshake.0 = Some(winner);
			} else {
				return Err(Error::<T>::HandshakeAlreadySet.into());
			}
		} else {
			if new_handshake.1 == None {
				new_handshake.1 = Some(winner);
			} else {
				return Err(Error::<T>::HandshakeAlreadySet.into());
			}
		}
		Ok(new_handshake)
	}
}
