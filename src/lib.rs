#![allow(static_mut_refs)]
#![no_std]
use gstd::{exec, msg, prelude::*};
use pebbles_game_io::*;

static mut GAME_STATE: Option<GameState> = None;

#[cfg(test)]
fn get_random_u32() -> u32 {
    2
}

#[cfg(not(test))]
fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

#[no_mangle]
extern "C" fn init() {
    let init_config: PebblesInit = msg::load().expect("Failed to decode PebblesInit");

    if init_config.pebbles_count == 0 || init_config.max_pebbles_per_turn == 0 {
        panic!("Invalid initialization parameters");
    }

    let first_player = if get_random_u32() % 2 == 0 {
        Player::User
    } else {
        Player::Program
    };

    let state = GameState {
        pebbles_count: init_config.pebbles_count,
        max_pebbles_per_turn: init_config.max_pebbles_per_turn,
        pebbles_remaining: init_config.pebbles_count,
        difficulty: init_config.difficulty,
        first_player: first_player.clone(),
        winner: None,
    };

    unsafe { GAME_STATE = Some(state) };

    if first_player == Player::Program {
        make_program_turn();
    }
}

#[no_mangle]
extern "C" fn handle() {
    let action: PebblesAction = msg::load().expect("Failed to decode PebblesAction");
    let state = get_state_mut();

    match action {
        PebblesAction::Turn(pebbles) => {
            if pebbles == 0
                || pebbles > state.max_pebbles_per_turn
                || pebbles > state.pebbles_remaining
            {
                panic!("Invalid turn");
            }

            // Execute player's turn
            state.pebbles_remaining -= pebbles;

            if state.pebbles_remaining == 0 {
                state.winner = Some(Player::User);
                msg::reply(PebblesEvent::Won(Player::User), 0).expect("Failed to send event");
                return;
            }

            // Execute program's turn
            make_program_turn();
        }
        PebblesAction::GiveUp => {
            state.winner = Some(Player::Program);
            msg::reply(PebblesEvent::Won(Player::Program), 0).expect("Failed to send event");
        }
        PebblesAction::Restart {
            difficulty,
            pebbles_count,
            max_pebbles_per_turn,
        } => {
            // Reset game state
            *state = GameState {
                pebbles_count,
                max_pebbles_per_turn,
                pebbles_remaining: pebbles_count,
                difficulty,
                first_player: if get_random_u32() % 2 == 0 {
                    Player::User
                } else {
                    Player::Program
                },
                winner: None,
            };

            if state.first_player == Player::Program {
                make_program_turn();
            }
        }
    }
}

fn make_program_turn() {
    let state = get_state_mut();
    let pebbles_to_remove = match state.difficulty {
        DifficultyLevel::Easy => {
            // Randomly choose number of pebbles to remove
            (get_random_u32() % state.max_pebbles_per_turn.min(state.pebbles_remaining)) + 1
        }
        DifficultyLevel::Hard => {
            // Use winning strategy
            calculate_winning_move(state.pebbles_remaining, state.max_pebbles_per_turn)
        }
    };

    state.pebbles_remaining -= pebbles_to_remove;

    if state.pebbles_remaining == 0 {
        state.winner = Some(Player::Program);
        msg::reply(PebblesEvent::Won(Player::Program), 0).expect("Failed to send event");
    } else {
        msg::reply(PebblesEvent::CounterTurn(pebbles_to_remove), 0).expect("Failed to send event");
    }
}

fn calculate_winning_move(remaining: u32, max_per_turn: u32) -> u32 {
    // Implement winning strategy
    let remainder = remaining % (max_per_turn + 1);
    if remainder == 0 {
        1
    } else {
        remainder
    }
}

#[no_mangle]
extern "C" fn state() {
    msg::reply(get_state().clone(), 0).expect("Failed to share state");
}

fn get_state_mut() -> &'static mut GameState {
    unsafe { GAME_STATE.as_mut().expect("Game state is not initialized") }
}

fn get_state() -> &'static GameState {
    unsafe { GAME_STATE.as_ref().expect("Game state is not initialized") }
}
