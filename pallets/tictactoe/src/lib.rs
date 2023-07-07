#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

///TBD: error: internal compiler error: encountered incremental compilation error with
/// mir_built(76e5305fbe3bf3e0-1cbbbe6365e28f21)
/// TBD: Move logic out of call ?
/// TBD: Put or mutate game in storage?
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// TBD: difference of using sp_runtime from frame_support or from sp_runtime directly
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	sp_runtime::{
		traits::{AccountIdConversion, Saturating, Zero},
		DispatchError,
	},
	traits::{Currency, ExistenceRequirement::KeepAlive, Get},
	PalletId, RuntimeDebug,
};

pub use pallet::*;

// pub mod weights;
// pub use weights::*;

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

		//type WeightInfo: WeightInfo;
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
		/// A safeguard deposit has been set.
		SafeguardDepositSet { deposit: BalanceOf<T> },
		/// A winner has been proposed.
		WinnerProposed { game_index: u32, winner: T::AccountId, proposer: T::AccountId },
		/// Mediation has been requested.
		MediationRequested { game_index: u32, proposer: T::AccountId },
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

			// let game = match Self::games(game_index) {
			// 	Some(game) => game,
			// 	None => return Err(Error::<T>::GameDoesNotExist.into()),
			// };
			// ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
			// ensure!(game.payout_addresses.1 == None, Error::<T>::GameFull);
			// let bet = game.bet;
			// let transfer_amount = bet.saturating_add(Self::safeguard_deposit());
			// T::Currency::transfer(&caller, &Self::account_id(), transfer_amount, KeepAlive)?;
			// let host = game.payout_addresses.0;
			// let new_game = Game {
			// 	bet: game.bet,
			// 	payout_addresses: (host, Some(caller.clone())),
			// 	ended: game.ended,
			// 	handshake: (None, None),
			// };
			//Games::<T>::insert(game_index, new_game);

			//TBD: Which one is better ? This or the commented out code above ?

			Games::<T>::try_mutate(game_index, |game| -> DispatchResult {
				let mut game = game.as_mut().ok_or_else(|| Error::<T>::GameDoesNotExist)?;
				ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
				ensure!(game.payout_addresses.1 == None, Error::<T>::GameFull);
				let bet = game.bet;
				let transfer_amount = bet.saturating_add(Self::safeguard_deposit());
				T::Currency::transfer(&caller, &Self::account_id(), transfer_amount, KeepAlive)?;
				game.payout_addresses.1 = Some(caller.clone());
				Ok(())
			})?;

			Self::deposit_event(Event::PlayerJoined { game_index, player: caller });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn end_game(
			origin: OriginFor<T>,
			game_index: u32,
			winner: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			//let _ = Self::do_end_game(caller, winner, game_index);

			// Retrieve game
			let game = match Self::games(game_index) {
				Some(game) => game,
				None => return Err(Error::<T>::GameDoesNotExist.into()),
			};

			// Check if game has ended
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);

			// Retrieve players
			let payout_addresses = game.payout_addresses;
			let host = payout_addresses.0.ok_or_else(|| Error::<T>::BadAddress)?;
			let joiner = payout_addresses.1.ok_or_else(|| Error::<T>::BadAddress)?;

			// Check if caller and proposed winner is a player
			ensure!(caller == host || caller == joiner, Error::<T>::NotAPlayer);
			ensure!(winner == host || winner == joiner, Error::<T>::NotAPlayer);

			// Update handshake and avoid writing to storage if already set

			let new_handshake = match Self::update_handshake(
				game.handshake,
				caller.clone(),
				host.clone(),
				winner.clone(),
			) {
				Ok(handshake) => handshake,
				Err(err) => return Err(err),
			};

			// Check if both players have agreed on the winner
			let mut winner_agreed = true;
			if new_handshake.0 == None || new_handshake.1 == None {
				Self::deposit_event(Event::WinnerProposed {
					game_index,
					winner: winner.clone(),
					proposer: caller.clone(),
				});
				winner_agreed = false;
			} else if new_handshake.0 != new_handshake.1 {
				Self::deposit_event(Event::MediationRequested { game_index, proposer: caller });
				winner_agreed = false;
			}

			// Update game and write to storage
			let new_game = Game {
				bet: game.bet,
				payout_addresses: (Some(host.clone()), Some(joiner.clone())),
				ended: winner_agreed.clone(),
				handshake: new_handshake,
			};
			Games::<T>::insert(game_index, new_game);

			if winner_agreed {
				// Both players have agreed on the winner, transfer jackpot and safeguard deposit
				let jackpot = game.bet.saturating_mul(2u32.into());
				let safeguard_deposit = Self::safeguard_deposit();

				// TBD: BatchTransfer? Better to add logic and add total transfer amount ?
				let _ = T::Currency::transfer(
					&Self::account_id(),
					&host,
					safeguard_deposit,
					KeepAlive,
				)?;
				let _ = T::Currency::transfer(
					&Self::account_id(),
					&joiner,
					safeguard_deposit,
					KeepAlive,
				)?;
				let _ = T::Currency::transfer(&Self::account_id(), &winner, jackpot, KeepAlive)?;
				Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });

				return Ok(())
			}

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
			Self::deposit_event(Event::SafeguardDepositSet { deposit });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn force_end_game(
			origin: OriginFor<T>,
			game_index: u32,
			winner: T::AccountId,
			deposit_benefiicary: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			let game = match Self::games(game_index) {
				Some(game) => game,
				None => return Err(Error::<T>::GameDoesNotExist.into()),
			};
			ensure!(!game.ended, Error::<T>::GameAlreadyEnded);

			let new_game = Game {
				bet: game.bet,
				payout_addresses: game.payout_addresses,
				ended: true,
				handshake: (Some(winner.clone()), Some(winner.clone())),
			};
			Games::<T>::insert(game_index, new_game);
			let jackpot = game.bet.saturating_mul(2u32.into());
			let safeguard_deposit = Self::safeguard_deposit();
			let _ = T::Currency::transfer(
				&Self::account_id(),
				&deposit_benefiicary,
				safeguard_deposit,
				KeepAlive,
			)?;
			let _ = T::Currency::transfer(&Self::account_id(), &winner, jackpot, KeepAlive)?;
			Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn withdraw_funds(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			beneficiary: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			T::Currency::transfer(&Self::account_id(), &beneficiary, amount, KeepAlive)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	fn update_handshake(
		handshake: (Option<T::AccountId>, Option<T::AccountId>),
		caller: T::AccountId,
		host: T::AccountId,
		winner: T::AccountId,
	) -> Result<(Option<T::AccountId>, Option<T::AccountId>), DispatchError> {
		// Update handshake and avoid writing to storage if already set
		let mut new_handshake = handshake;
		if caller == host {
			if new_handshake.0 == None {
				new_handshake.0 = Some(winner.clone());
			} else {
				return Err(Error::<T>::HandshakeAlreadySet.into())
			}
		} else {
			if new_handshake.1 == None {
				new_handshake.1 = Some(winner.clone());
			} else {
				return Err(Error::<T>::HandshakeAlreadySet.into())
			}
		}
		Ok(new_handshake)
	}
}
