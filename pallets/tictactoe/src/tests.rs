use crate::{mock::*, Error, Event, GameIndex};
use frame_support::{assert_noop, assert_ok};

#[test]
fn initial_state() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(Tictactoe::account_id()), 0);
        assert_eq!(Tictactoe::game_index(), 0);
        assert!(crate::Games::<Test>::get(0).is_none());
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
        assert_eq!(crate::Games::<Test>::get(0).unwrap().bet, Some(bet));
        assert_eq!(crate::Games::<Test>::get(0).unwrap().jackpot, Some(bet));
        assert_eq!(crate::Games::<Test>::get(0).unwrap().payout_addresses, (creator, creator));
        assert_eq!(crate::Games::<Test>::get(0).unwrap().ended, false);
        assert_eq!(Balances::free_balance(Tictactoe::account_id()), bet)

	});
}
