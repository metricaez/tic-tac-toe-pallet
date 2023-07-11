use crate::{mock::*, Error, Event};
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
fn set_safeguard_works() {
	new_test_ext().execute_with(|| {
		// Since checking events, we need to set block number for events to be deposited.
		System::set_block_number(1);
		let safeguard_deposit = 1;
		assert_ok!(Tictactoe::set_safeguard_deposit(RuntimeOrigin::root(), safeguard_deposit));
		System::assert_last_event(
			(Event::SafeguardDepositSet { deposit: safeguard_deposit }).into(),
		);
		assert_eq!(Tictactoe::safeguard_deposit(), safeguard_deposit);
	});
}

#[test]
fn set_safeguarde_fails_without_root() {
	new_test_ext().execute_with(|| {
		let safeguard_deposit = 1;
		assert!(
			Tictactoe::set_safeguard_deposit(RuntimeOrigin::signed(1), safeguard_deposit).is_err()
		);
	});
}

#[test]
fn create_game_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let host = 1;
		let initial_balance = Balances::free_balance(&host);
		let bet = 10;
		let safeguard_deposit = 1;

		// Set safeguard deposit to check that is correctly transferred to pallet account
		assert_ok!(Tictactoe::set_safeguard_deposit(RuntimeOrigin::root(), safeguard_deposit));
		// Create game
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		// Check that event was emitted. First game created, so index is 0.
		System::assert_last_event((Event::GameCreated { game_index: 0 }).into());
		// Check that host balance was correctly updated
		assert_eq!(Balances::free_balance(&host), initial_balance - bet - safeguard_deposit);
		// Check that game_index was correctly incremented.
		assert_eq!(Tictactoe::game_index(), 1);
		// Check created game instance. Game_id = 0 since first game created.
		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(host), None));
		assert_eq!(Tictactoe::games(0).unwrap().ended, false);
		// Check that funds were correctly transferred to pallet account.
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), bet + safeguard_deposit);
	});
}

#[test]
fn create_game_fails_with_zero_bet() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let initial_balance = Balances::free_balance(&host);
		let bet = 0;
		assert_noop!(
			Tictactoe::create_game(RuntimeOrigin::signed(host), bet),
			Error::<Test>::CantBeZero
		);
		// Check that no state was changed after failed extrinsic.
		assert!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet).is_err());
		assert_eq!(Balances::free_balance(&host), initial_balance);
		assert_eq!(Tictactoe::game_index(), 0);
		assert!(Tictactoe::games(0).is_none());
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), 0);
	});
}

#[test]
fn create_game_fails_insufficient_funds() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let initial_balance = Balances::free_balance(&host);
		let bet = initial_balance + 1;
		assert!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet).is_err());
	});
}

#[test]
fn join_a_game_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let host = 1;
		let joiner = 2;
		let bet = 10;
		let safeguard_deposit = 1;
		assert_ok!(Tictactoe::set_safeguard_deposit(RuntimeOrigin::root(), safeguard_deposit));
		// To join a game, it must be created first.
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));

		let initial_balance = Balances::free_balance(&joiner);
		// Game id = 0 since first game created
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		System::assert_last_event((Event::PlayerJoined { game_index: 0, player: joiner }).into());
		// Check that storage and balances were correctly updated.
		assert_eq!(Balances::free_balance(&joiner), initial_balance - bet - safeguard_deposit);
		assert_eq!(Tictactoe::game_index(), 1);
		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(host), Some(joiner)));
		assert_eq!(Tictactoe::games(0).unwrap().ended, false);
		// Pallet must hold two bets and two safeguard deposits since only one game is created.
		assert_eq!(
			Balances::free_balance(Tictactoe::account_id()),
			(bet * 2) + (safeguard_deposit * 2)
		);
	});
}

#[test]
fn join_a_non_existent_game_fails() {
	new_test_ext().execute_with(|| {
		let joiner = 2;
		assert_noop!(
			Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0),
			Error::<Test>::GameDoesNotExist
		);
		assert!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0).is_err());
	});
}

#[test]
fn join_a_full_game_fails() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let joiner = 2;
		let malicious_joiner = 3;
		let bet = 10;
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		assert_noop!(
			Tictactoe::join_game(RuntimeOrigin::signed(malicious_joiner), 0),
			Error::<Test>::GameFull
		);
	});
}

#[test]
fn join_games_without_funds_fails() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let joiner = 2;
		let joiner_balance = Balances::free_balance(&joiner);
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(joiner),
			Tictactoe::account_id(),
			joiner_balance - 5
		));
		let bet = 10;
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0).is_err());
	});
}

#[test]
fn end_game_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		//Fund pallet account
		let pallet_funding = 50;
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(3),
			Tictactoe::account_id(),
			pallet_funding
		));

		let host = 1;
		let joiner = 2;
		let bet: u64 = 10;
		let safeguard_deposit = 1;

		// To end a game, it must be created and joined first.
		assert_ok!(Tictactoe::set_safeguard_deposit(RuntimeOrigin::root(), safeguard_deposit));
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));

		let host_init_balance = Balances::free_balance(&host);
		let joiner_init_balance = Balances::free_balance(&joiner);

		assert_eq!(Tictactoe::games(0).unwrap().handshake, (None, None));

		let proposed_winner = host;

		// Host proposes a winner, an event is emitted and handshake is updated.
		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(host), 0, proposed_winner));
		System::assert_last_event(
			(Event::WinnerProposed { game_index: 0, winner: proposed_winner, proposer: host })
				.into(),
		);

		// Joiner proposes same winner, since consensus is reached, game is ended.
		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(joiner), 0, proposed_winner));
		System::assert_last_event(
			(Event::GameEnded { game_index: 0, winner: proposed_winner, jackpot: bet * 2 }).into(),
		);

		// Check that balances and storage were correctly updated.
		assert_eq!(Balances::free_balance(&host), host_init_balance + safeguard_deposit + bet * 2);
		assert_eq!(Balances::free_balance(&joiner), joiner_init_balance + safeguard_deposit);

		assert_eq!(Tictactoe::games(0).unwrap().bet, bet);
		assert_eq!(Tictactoe::games(0).unwrap().payout_addresses, (Some(host), Some(joiner)));
		assert_eq!(Tictactoe::games(0).unwrap().ended, true);
		assert_eq!(Tictactoe::games(0).unwrap().handshake, (Some(host), Some(host)));

		assert_eq!(Balances::free_balance(Tictactoe::account_id()), pallet_funding);

		let new_joiner = 4;
		assert_noop!(
			Tictactoe::join_game(RuntimeOrigin::signed(new_joiner), 0),
			Error::<Test>::GameAlreadyEnded
		);
	});
}

#[test]
fn end_a_game_with_one_player_fails() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let bet: u64 = 10;
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_noop!(
			Tictactoe::end_game(RuntimeOrigin::signed(host), 0, host),
			Error::<Test>::BadAddress
		);
	});
}

#[test]
fn mediation_is_applied() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		//Fund pallet account
		let pallet_funding = 50;
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(3),
			Tictactoe::account_id(),
			pallet_funding
		));

		let host = 1;
		let joiner = 2;
		let host_init_balance = Balances::free_balance(&host);
		let joiner_init_balance = Balances::free_balance(&joiner);
		let bet: u64 = 10;
		let safeguard_deposit = 1;

		assert_ok!(Tictactoe::set_safeguard_deposit(RuntimeOrigin::root(), safeguard_deposit));
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));

		let host_proposed_winner = host;
		let joiner_proposed_winner = joiner;

		// For mediation to be applied, both players must propose different winners.
		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(host), 0, host_proposed_winner));
		assert_ok!(Tictactoe::end_game(RuntimeOrigin::signed(joiner), 0, joiner_proposed_winner));
		System::assert_last_event(
			(Event::MediationRequested { game_index: 0, proposer: joiner }).into(),
		);

		assert_eq!(Balances::free_balance(&host), host_init_balance - bet - safeguard_deposit);
		assert_eq!(Balances::free_balance(&joiner), joiner_init_balance - bet - safeguard_deposit);

		// Assuming host was correct. Force end game on favor of host.
		assert_ok!(Tictactoe::force_end_game(RuntimeOrigin::root(), 0, host_proposed_winner, host));
		System::assert_last_event(
			(Event::GameEnded { game_index: 0, winner: host_proposed_winner, jackpot: bet * 2 })
				.into(),
		);

		assert_eq!(Balances::free_balance(&host), host_init_balance + bet);
		assert_eq!(Balances::free_balance(&joiner), joiner_init_balance - bet - safeguard_deposit);
		assert_eq!(
			Balances::free_balance(Tictactoe::account_id()),
			pallet_funding + safeguard_deposit
		);
	});
}

#[test]
fn invalid_accounts_fail_to_end() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let joiner = 2;
		let invalid_account = 3;
		let bet: u64 = 10;
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		assert_noop!(
			Tictactoe::end_game(RuntimeOrigin::signed(invalid_account), 0, host),
			Error::<Test>::NotAPlayer
		);
		assert_noop!(
			Tictactoe::end_game(RuntimeOrigin::signed(host), 0, invalid_account),
			Error::<Test>::NotAPlayer
		);
	});
}

#[test]
fn non_sudo_cant_force_end() {
	new_test_ext().execute_with(|| {
		let host = 1;
		let joiner = 2;
		let bet: u64 = 10;
		assert_ok!(Tictactoe::create_game(RuntimeOrigin::signed(host), bet));
		assert_ok!(Tictactoe::join_game(RuntimeOrigin::signed(joiner), 0));
		assert!(Tictactoe::force_end_game(RuntimeOrigin::signed(host), 0, host, host).is_err());
	});
}

#[test]
fn withdraw_funds_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		//Fund pallet account
		let pallet_funding = 50;
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(3),
			Tictactoe::account_id(),
			pallet_funding
		));

		let beneficiary = 1;
		let amount: u64 = 10;

		assert_ok!(Tictactoe::withdraw_funds(RuntimeOrigin::root(), amount, beneficiary));
		System::assert_last_event((Event::FundsWithdrawn { amount, beneficiary }).into());
	});
}

#[test]
fn non_sudo_cant_withdraw() {
	new_test_ext().execute_with(|| {
		//Fund pallet account
		let pallet_funding = 50;
		assert_ok!(Balances::transfer(
			RuntimeOrigin::signed(3),
			Tictactoe::account_id(),
			pallet_funding
		));
		let beneficiary = 1;
		let amount: u64 = 10;
		assert!(Tictactoe::withdraw_funds(RuntimeOrigin::signed(beneficiary), amount, beneficiary)
			.is_err());
	});
}
