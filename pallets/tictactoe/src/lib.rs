#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{traits::Currency, PalletId, RuntimeDebug};

pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Clone, Encode, Decode, Default, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Game<Balance, AccountId> {
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
		GameCreated{game_index:u32},
		/// A player has won a game.
		GameWon{winner:T::AccountId, jackpot:BalanceOf<T>},
	}

	#[pallet::error]
	pub enum Error<T> {
		
	}

	#[pallet::storage]
	#[pallet::getter(fn game_index)]
	pub(crate) type GameIndex<T> = StorageValue<_, u32, ValueQuery>;
 
	#[pallet::storage]
	#[pallet::getter(fn games)]
	pub(crate) type Games<T: Config> = StorageMap<_, Twox64Concat, u32, Game<BalanceOf<T>,T::AccountId>, OptionQuery>;
	
}
