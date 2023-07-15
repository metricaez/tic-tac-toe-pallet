# TicTacToe pallet

## Overview

A tic-tac-toe pallet that works as a proof of concept for a secure jackpot storage while games are being played.

With this pallet, two players can agree to play a game while the pallet holds a certain amount of funds from each one of them called “bet”, the winner of the game will take the “jackpot: which consist of both his bet and the opponent bet.

A player creates a game with “create_game” call and set the bet value, this amount will be transferred into the pallet and held while the game is being disputed. This player is referred as the “host”

Another player can join an existing game with “join_game”, it must have enough funds to pay for the bet amount set by the game creator. This player is referred as the “joiner”.

Both the host and the joiner must also deposit a safeguard deposit to be slashed in case of bad behavior.

Game logic is not tracked on chain so the winner must be stated when finishing a game, to avoid users closing games in a malicious way, both the host and the joiner must propose a winner. If the proposed winners match, jackpot is sent to that winner and safeguard deposit are released and game is automatically ended. If the proposed winners do not match, a root account is able to force-end a game, it is assumed that this root user is a trusted user that can review the game logic and history and decide who the legitimate winner is.

Safeguard deposit can be slashed from the player that proposed the wrong winner, slashed funds are held in the pallet are can be withdrawn by the sudo user.

:warning: It is **not a production-ready pallet**, but a sample built for learning purposes. It is discouraged to use this code 'as-is' in a production runtime.

## Glossary

* **Game** – Both used to refer to a game instance and the `struct` that stores the relevant data of said instance, for example if it has ended the player accounts.
* **Player** – An user playing a game instance.
* **Host** – The player that creates a game.
* **Joiner** – The player that joins an already created game.
* **Admin** – Account with admin privileges that is able to execute certain calls that other users can't. Also referred as `root` or `sudo`
* **Bet** – Amount of `Currency`, therefore `Balance`, that user must stake to participate in a game.
* **Jackpot** – Amount of `Currency` that the winner a game will get, it is composed of it's own `bet` and the one staked by the opponent. 
* **Safeguard Deposit** – A fixed amount set by the **admin** to be deposited as safeguard deposit while playing a game, if correctly ended with no mediation requirde, it is returned to their respective depositor accounts. 
* **Vault** – The pallet account, which secures and holds funds of players playing games.
* **Handshake** – A tuple of accounts that is used for checking the proposed winner that each player declares. 

## Configuration

### Types
* `RuntimeEvent` – The overarching event type.
* `Currency` – The currency type.
* `WeightInfo` – Information on runtime weights.

### Constants
* `PalletId` – Pallet ID. Used for account derivation.

## Storage
* `GameIndex` – Stores the index of the new *Game* to be created. Increments on each game creation. 
	* `StorageValue<u32, ValueQuery>`
	* Getter – ```fn game_index()```
* `SafeguardDeposit` – Stores the value of the safeguard deposit that players must deposit to join a game. 
	* `StorageValue<Balance, ValueQuery>`
	* Getter – ```fn safeguard_deposit()```
* `Games` - Map that store all the game instances and tracks their states.
	* `StorageMap<u32, Game, ValueQuery>`
	* Getter – ```fn games(u32)```
## Extrinsics

<details>
<summary><h3>create_game</h3></summary>

Create a new game instance.
* Transfer `bet` and `safeguard` deposit to vault. 
* Set the bet value for other player to stake to join the game. 
* Caller is set as `host` of the game. 

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `bet` – Amount of `Currency` to be transferred from the caller account to the vault. It is also set as the value that a joiner must transfer to join. Can't be zero.
#### Events:
* Emits `GameCreated` with the `game_index` of the created game as parameter on success.
#### Errors:
  * `CantBeZero` – `bet` was passed with zero as value.
  * `IndexOverflow` – The game index overflows while trying to be incremented.
  * All Errors from `Currency::transfer` apply.
</details>

<details>
<summary><h3>join_game</h3></summary>

Join an existing game by it's index.
* Game must have been created.
* The game must not have finished and must not be full.
* Transfer `bet` and `safeguard` deposit to vault. 
* Bet value for the game has already been set by creator. 
* Caller is set as `joiner` of the game. 


#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `game_index` – Index of the game to join. Must be previously known by caller.

#### Events:
* Emits `PlayerJoined` with the `game_index` of the joined game and the `player` that joined as parameters on success.

#### Errors:
  * `GameDoesNotExist` – No game exist for the passed `game_index`
  * `GameAlreadyEnded` - The game with passed `game_index` has already ended.
  * `GameFull` - A player has already joined the game with the passed `game_index`
  *   All Errors from `Currency::transfer` apply.
</details>

<details>
<summary><h3>end_game</h3></summary>

Try to end a game by it's index by proposing a winner.
* Game must have been created.
* The game must not have finished and must be full.
* The calling account and proposed winner must be players of the game.
* A `winner` account is proposed. 
	* If extrinsic being called for the first time on a game, the account is proposed as winner.
	* If extrinsic has already been called by the other player:
		* If consensus on who the winner is is achieve, the game ends.
		* Otherwise, mediation is called. 
* If game is ended, jackpot is transferred to winner and safeguard returned to their owners.

#### Parameters:
  * `origin` – Origin for the call. Must be signed.
  * `game_index` – Index of the game to join. Must be previously known by caller.
  * `winner` – Account proposed as winner of the game. Game logic and result must be known by the caller.

#### Events:
* Emits `GameEnded` with the following parameter on successfully ending a game: 
	* `game_index` of the ended game. 
	* `winner` the winner account.
	* `jackpot` as the amount sent jackpot to the winner.
* Emits `WinnerProposed` when successfully called for the first time by a player with the following parameters:
	* `game_index` of the game in which the winner was proposed.
	* `winner` as the proposed winner.
	* `proposer` as the caller that proposed said winner.
* Emits `MediationRequested` when successfully called but the proposed winner of both players doesn't match. Parameters:
	* `game_index` of the game in which the winner was proposed.
	* `proposer` as the caller that proposed the winner that set the disagreement.

#### Errors:
  * `GameDoesNotExist` – No game exist for the passed `game_index`
  * `GameAlreadyEnded` - The game with passed `game_index` has already ended.
  * `BadAddress` - Error while reading the player accounts stored for the game instance or one of them is `None` 
  * `HandshakeAlreadySet` - A player is trying to re-propose a winner.
  *   All Errors from `Currency::transfer` apply.
</details>

<details>
<summary><h3>set_safeguard_deposit</h3></summary>

Change value of `SafeguardDeposit`.

#### Parameters:
  * `origin` – Origin for the call. Must be signed and `root`.
  * `deposit` – Amount to be set.
#### Events:

* Emits `SafeguardDepositSet` on success with `deposit` amount set as the parameter.
 
</details>

<details>
<summary><h3>force_end_game</h3></summary>

Force end a game.
* Must be called by **admin**.
* Closes the game and transfer jackpot to designed winner.
* Only one `safeguard deposit`s is returned to a player, slashing this amount from the other as penalization assuming bad behavior.

#### Parameters:
  * `origin` – Origin for the call. Must be signed and `root`.
  * `game_index` – Index of the game to join. Must be previously known by caller.
  * `winner` – Decided winner account. It will receive the `jackpot`
  * `deposit_beneficiary` – Account that will have its`safeguard deposit` returned.
#### Events:
* Emits `GameEnded` on success with the following parameters:
	* `game_index` of the ended game. 
	* `winner` the winner account.
	* `jackpot` as the amount sent jackpot to the winner.

#### Errors:
  * `GameAlreadyEnded` - The game with passed `game_index` has already ended.
  * All Errors from `Currency::transfer` apply.
</details>

<details>
<summary><h3>withdraw_funds</h3></summary>

Withdraw a certain amount of funds from vault to a beneficiary account.

#### Parameters:
  * `origin` – Origin for the call. Must be signed and `root`.
  * `amount` – Amount to be withdrawn.
  * `beneficiary` – Account that will receive the funds.
#### Events:
* Emits `FundsWithdrawn` on success with the `amount` and `beneficiary` as parameters.

#### Errors:
  * All Errors from `Currency::transfer` apply.
</details>

## RPC

<details>
<summary><h3>get_currency_to_asset_output_amount</h3></summary>

Get the output amount for a fixed-input currency-to-asset trade,
i.e. 'How much asset would I get if I paid this much currency'?

#### Parameters:
* `asset_id` – ID of the asset to be bought.
* `currency_amount` – The amount of currency to be spent.
</details>

<details>
<summary><h3>get_currency_to_asset_input_amount</h3></summary>

Get the input amount for a fixed-output currency-to-asset trade,
i.e. 'How much currency do I have to pay to get this much asset'?

#### Parameters:
* `asset_id` – ID of the asset to be bought.
* `token_amount` – The amount of tokens to be bought.
</details>

<details>
<summary><h3>get_asset_to_currency_output_amount</h3></summary>

Get the output amount for a fixed-input asset-to-currency trade,
i.e. 'How much currency would I get if I paid this much asset'?

#### Parameters:
* `asset_id` – ID of the asset to be sold.
* `token_amount` – The amount of tokens to be spent.
</details>

<details>
<summary><h3>get_asset_to_currency_input_amount</h3></summary>
Get the input amount for a fixed-output currency-to-asset trade,
i.e. 'How much asset do I have to pay to get this much currency'?

#### Parameters:
* `asset_id` – ID of the asset to be sold.
* `token_amount` – The amount of currency to be bought.
</details>

### Errors (for all methods):
* `ExchangeNotFound` – There is no exchange for the given `asset_id`.
* `NotEnoughLiquidity` – There is not enough liquidity in the pool to buy the specified amount of asset/currency.
  (applies only to fixed-output price queries).
* `Overflow` – An overflow occurred during price computation.
* `Unexpected` – An unexpected runtime error occurred.

## How to add `pallet-dex` to a node

:information_source: The pallet is compatible with Substrate version
[polkadot-v0.9.46](https://github.com/paritytech/polkadot/releases/tag/v0.9.43).

:information_source: This section is based on
[Substrate node template](https://github.com/substrate-developer-hub/substrate-node-template/).
Integrating `pallet-tictactoe` with another node might look slightly different.

:information_source: An implementation of `pallet-tictactoe` in a `substrate-node-template` can be found [here](https://github.com/metricaez/tic-tac-toe-pallet).

### Runtime's `Cargo.toml`

Add `pallet-tictactoe`, to dependencies:
```toml
[dependencies]
#--snip--
pallet-tictactoe = { version = "0.1.0-dev", default-features = false, path = "../pallets/tictactoe" }
#--snip--
```

Update the runtime's `std` feature:
```toml
std = [
    # --snip--
    "pallet-tictactoe/std",
    # --snip--
]
```
### Runtime's `lib.rs`

Import required types and traits.
```rust
use frame_support::PalletId;
```
Configure the tictactoe pallet.
```rust

parameter_types! {

pub  const  TictactoePalletId:  PalletId  =  PalletId(*b"py/tctct");

}

// Configure the tictactoe pallet.
impl  pallet_tictactoe::Config  for  Runtime {

type  RuntimeEvent  =  RuntimeEvent;

type  PalletId  =  TictactoePalletId;

type  Currency  =  Balances;

type  WeightInfo  =  pallet_tictactoe::weights::SubstrateWeight<Runtime>;

}
```

Add configured pallets to the `construct_runtime` macro call.
```rust
construct_runtime!(
    pub enum Runtime where
        // --snip--
    {
        // --snip---
        Tictactoe: pallet_tictactoe,
        // --snip---
    }
);
```

Add the RPC implementation.
```rust
impl_runtime_apis! {
    // --snip--
    impl pallet_dex_rpc_runtime_api::DexApi<Block, AssetId, Balance, AssetBalance> for Runtime {
        fn get_currency_to_asset_output_amount(
            asset_id: AssetId,
            currency_amount: Balance
        ) -> pallet_dex_rpc_runtime_api::RpcResult<AssetBalance> {
            Dex::get_currency_to_asset_output_amount(asset_id, currency_amount)
        }

        fn get_currency_to_asset_input_amount(
            asset_id: AssetId,
            token_amount: AssetBalance
        ) -> pallet_dex_rpc_runtime_api::RpcResult<Balance> {
            Dex::get_currency_to_asset_input_amount(asset_id, token_amount)
        }

        fn get_asset_to_currency_output_amount(
            asset_id: AssetId,
            token_amount: AssetBalance
        ) -> pallet_dex_rpc_runtime_api::RpcResult<Balance> {
            Dex::get_asset_to_currency_output_amount(asset_id, token_amount)
        }

        fn get_asset_to_currency_input_amount(
            asset_id: AssetId,
            currency_amount: Balance
        ) -> pallet_dex_rpc_runtime_api::RpcResult<AssetBalance> {
            Dex::get_asset_to_currency_input_amount(asset_id, currency_amount)
        }
    }
}
```
## Frontend

A simple React-App that allows to play a TicTacToe game and interact with the pallet running on a Node based on Substrate Template can be found [here](https://github.com/metricaez/tic-tac-toe-dapp).
## Integration to Polkadot parachain

A Cumulus based parachain built from [Substrate Parachain Template](https://github.com/substrate-developer-hub/substrate-parachain-template) can be found [here](https://github.com/metricaez/tictactoe-integration). It is particularly useful for integration testing based on [Zombienet](https://github.com/paritytech/zombienet).
## Improvement Proposals

Here are some suggested interatios recommended for this project:
* The current mechanism for conflict resolution is extremely centralized with all the risks that this implies, so moving from `sudo` into some form of collective decision making or system, like democracy, would be suggested.
* Following with the same concept, implementing a reputation system would be a great feature, creating different ranges in which players with the same reputations are matched. This mechanism is extremely popular in online gaming and would  ideally push good actors to play with good actors and vice versa with the bad ones. 