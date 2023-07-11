

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

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

pub mod weights;
pub use weights::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(
	Clone, Encode, Decode, Default, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]

/// Game struct
pub struct Game<Balance, AccountId> {
	// Bet amount to join the game. Jackpot will be 2x bet amount.
	bet: Balance,
	// Stores the payout addresses of the host and joiner.
	payout_addresses: (Option<AccountId>, Option<AccountId>),
	// Indicates if the game has ended.
	ended: bool,
	// Stores the handshake between host and joiner to agree on the winner.
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

		type WeightInfo: WeightInfo;
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
		/// Funds has been withdrawn.
		FundsWithdrawn { amount: BalanceOf<T>, beneficiary: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The game does not exist.
		GameDoesNotExist,
		/// The game index has overflowed.
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

	/// Index to identify each game.
	#[pallet::storage]
	#[pallet::getter(fn game_index)]
	pub(crate) type GameIndex<T> = StorageValue<_, u32, ValueQuery>;

	/// Safeguard deposit value to be used in case of dispute.
	#[pallet::storage]
	#[pallet::getter(fn safeguard_deposit)]
	pub(crate) type SafeguardDeposit<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Storage for game instances.
	#[pallet::storage]
	#[pallet::getter(fn games)]
	pub(crate) type Games<T: Config> =
		StorageMap<_, Twox64Concat, u32, Game<BalanceOf<T>, T::AccountId>, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Create a new game.
		/// The caller will be the host of the game.
		/// The bet amount will set the value to other user to join the game.
		/// Bet amount and safeguard deposit will be transferred to the pallet account.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_game())]
		pub fn create_game(origin: OriginFor<T>, bet: BalanceOf<T>) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			ensure!(!bet.is_zero(), Error::<T>::CantBeZero);

			// Transfer bet amount and safeguard deposit to pallet account to ensure creator account has enough funds.
			let transfer_amount = bet.saturating_add(Self::safeguard_deposit());
			T::Currency::transfer(&caller, &Self::account_id(), transfer_amount, KeepAlive)?;

			// Create new game and write to storage
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

		/// Join a game by it's index.
		/// The caller will be the joiner of the game.
		/// The alredy set bet and safeguard deposit amount will be transferred to the pallet account.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::join_game())]
		pub fn join_game(origin: OriginFor<T>, game_index: u32) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;

			// Retrieve game and update payout address if joiner account has enough funds.
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

		/// End a game by it's index.
		/// Game ends when both players agree on the winner or when root forces the end of the game.
		/// Expected to be called by the two players of the game.
		/// Each caller proposes a winner. If they match jackpot is sent, otherwise mediation is requested.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::end_game())]
		pub fn end_game(
			origin: OriginFor<T>,
			game_index: u32,
			winner: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

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
				// A winner has been proposed, pending for the other player to propose.
				Self::deposit_event(Event::WinnerProposed {
					game_index,
					winner: winner.clone(),
					proposer: caller.clone(),
				});
				winner_agreed = false;
			} else if new_handshake.0 != new_handshake.1 {
				// Both players have proposed a winner, but they don't match.
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

				// Transfer funds.
				Self::transfer_from_pallet(host, safeguard_deposit)?;
				Self::transfer_from_pallet(joiner, safeguard_deposit)?;
				Self::transfer_from_pallet(winner.clone(), jackpot)?;

				Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });

				return Ok(())
			}

			Ok(())
		}

		/// Set the safeguard deposit value.
		/// Only root can set this value.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_safeguard_deposit())]
		pub fn set_safeguard_deposit(
			origin: OriginFor<T>,
			deposit: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			SafeguardDeposit::<T>::put(deposit);
			Self::deposit_event(Event::SafeguardDepositSet { deposit });
			Ok(())
		}

		/// Force end a game by it's index.
		/// Only root can force end a game.
		/// The winner and deposit beneficiary will receive the jackpot and safeguard deposit respectively.
		/// The game will be marked as ended.
		/// This function is expected to be called in case of dispute and game logic must be handled off-chain.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::force_end_game())]
		pub fn force_end_game(
			origin: OriginFor<T>,
			game_index: u32,
			winner: T::AccountId,
			deposit_benefiicary: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			
			Games::<T>::try_mutate(game_index, |game| -> DispatchResult {
				let mut game = game.as_mut().ok_or_else(|| Error::<T>::GameDoesNotExist)?;
				ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
				
				// Update and end game. Set handshake to signal decision.
				game.ended = true;
				game.handshake = (Some(winner.clone()), Some(winner.clone()));

				// Transfer jackpot and safeguard deposit, bad actor account will not receive the safeguard deposit.
				let jackpot = game.bet.saturating_mul(2u32.into());
				let safeguard_deposit = Self::safeguard_deposit();
				Self::transfer_from_pallet(deposit_benefiicary.clone(), safeguard_deposit)?;
				Self::transfer_from_pallet(winner.clone(), jackpot)?;
				
				Self::deposit_event(Event::GameEnded { game_index, winner, jackpot });
				Ok(())
			})?;
			Ok(())
		}

		/// Withdraw funds from the pallet account.
		/// Only root can withdraw funds.
		/// Intended to be used to withdraw funds left from slashed accounts.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::withdraw_funds())]
		pub fn withdraw_funds(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			beneficiary: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::transfer_from_pallet(beneficiary.clone(), amount)?;
			Self::deposit_event(Event::FundsWithdrawn { amount, beneficiary });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	
	/// Returns the pallet account id.
	/// Store in variable to avoid calling the function multiple times.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Send funds from the pallet account to a beneficiary.
	fn transfer_from_pallet(
		beneficiary: T::AccountId,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		T::Currency::transfer(&Self::account_id(), &beneficiary, amount, KeepAlive)?;
		Ok(())
	}

	/// Update handshake and avoid writing to storage if already set
	/// Host proposed winner is store in handshake.0
	/// Joiner proposed winner is store in handshake.1
	fn update_handshake(
		handshake: (Option<T::AccountId>, Option<T::AccountId>),
		caller: T::AccountId,
		host: T::AccountId,
		winner: T::AccountId,
	) -> Result<(Option<T::AccountId>, Option<T::AccountId>), DispatchError> {
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
