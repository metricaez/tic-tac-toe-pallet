use crate::{mock::*, Error, Event, GameIndex};
use frame_support::{assert_noop, assert_ok};

#[test]
fn initial_state() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), 0);
		assert_eq!(Tictactoe::game_index(), 0);
		assert_eq!(Tictactoe::safeguard_deposit(), 0);
		assert!(Tictactoe::games(0).is_none());
	});
}

#[test]
fn create_game_works() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let initial_balance = Balances::free_balance(&creator);
		let bet = 10;
		assert_ok!(Tictactoe::start_game(RuntimeOrigin::signed(creator), bet));
		assert_eq!(Balances::free_balance(&creator), initial_balance - bet);
		assert_eq!(Tictactoe::game_index(), 1);
		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(creator), None));
		assert_eq!(Tictactoe::games(0).unwrap().ended, false);
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), bet)
	});
}

#[test]
fn create_game_fails_with_zero_bet() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let initial_balance = Balances::free_balance(&creator);
		let bet = 0;
		assert_noop!(
			Tictactoe::start_game(RuntimeOrigin::signed(creator), bet),
			Error::<Test>::CantBeZero
		);
		assert!(Tictactoe::start_game(RuntimeOrigin::signed(creator), bet).is_err());
		assert_eq!(Balances::free_balance(&creator), initial_balance);
		assert_eq!(Tictactoe::game_index(), 0);
		assert!(Tictactoe::games(0).is_none());
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), 0);
	});
}

#[test]
fn join_a_game_works() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let joiner = 2;
		let bet = 10;
		assert_ok!(Tictactoe::start_game(RuntimeOrigin::signed(creator), bet));

		let initial_balance = Balances::free_balance(&joiner);
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		assert_eq!(Balances::free_balance(&joiner), initial_balance - bet);
		assert_eq!(Tictactoe::game_index(), 1);
		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(creator), Some(joiner)));
		assert_eq!(Tictactoe::games(0).unwrap().ended, false);
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), bet * 2);
	});
}

#[test]
fn join_a_full_game_fails() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let joiner = 2;
		let malicious_joiner = 3;
		let bet = 10;
		assert_ok!(Tictactoe::start_game(RuntimeOrigin::signed(creator), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		assert!(Tictactoe::join_game(RuntimeOrigin::signed(malicious_joiner), 0).is_err());
	});
}

#[test]
fn end_game_works() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		//Fund pallet account
		let pallet_funding = 50;
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(3),
			Tictactoe::account_id(),
			pallet_funding
		));
		let creator = 1;
		let joiner = 2;
		let bet = 10;
		assert_ok!(Tictactoe::start_game(RuntimeOrigin::signed(creator), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));

		//TBD: Tooling for debugging and printing rather than only assertions ? How chan I see emited errors or events

		let initial_balance = Balances::free_balance(&creator);

		assert_eq!(Tictactoe::games(0).unwrap().handshake, (None, None));

		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(creator), 0, creator));

		// Print stored payout addresses
		println!("Payout addresses: {:?}", Tictactoe::games(0).unwrap().payout_addresses);
		println!("Handshake after player 1 calls ext: {:?}", Tictactoe::games(0).unwrap().handshake);

		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(joiner), 0, creator));

		// Print stored payout addresses
		println!("Payout addresses: {:?}", Tictactoe::games(0).unwrap().payout_addresses);
		println!("Handshake after player 2 calls ext : {:?}", Tictactoe::games(0).unwrap().handshake);

		assert_eq!(Tictactoe::games(0).unwrap().handshake, (Some(creator), Some(creator)));

		assert_eq!(Balances::free_balance(&creator), initial_balance + bet * 2);
		assert_eq!(Tictactoe::game_index(), 1);
		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(creator), Some(joiner)));
		assert_eq!(Tictactoe::games(0).unwrap().ended, true);
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), pallet_funding);
	});
}
