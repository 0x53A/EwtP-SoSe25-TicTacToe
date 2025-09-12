//! Rust bindings for MATLAB generated TicTacToe code
#![no_std]

// Include the generated bindings
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// Safe Rust wrapper functions
use core::mem::MaybeUninit;

/// Initialize the TicTacToe library
pub fn initialize() {
    unsafe { bindings::tic_tac_toe_initialize() };
}

/// Terminate the TicTacToe library
pub fn terminate() {
    unsafe { bindings::tic_tac_toe_terminate() };
}

pub type TicTacToeInput = bindings::struct0_T;
pub type TicTacToeOutput = bindings::struct1_T;

/// Make a move in the TicTacToe game
///
/// # Arguments
/// * `current_state` - The current state of the board as a 9-element array
/// * `player_turn` - The current player (1 or 2)
/// * `proposed_move` - The proposed move position (1-indexed, values from 1 to 9)
///
/// # Returns
/// A tuple containing:
/// * Whether the move was legal
/// * The new state of the board
/// * The next player's turn
pub fn make_move(input: TicTacToeInput) -> TicTacToeOutput {
    // Create output structure
    let mut output = MaybeUninit::<TicTacToeOutput>::uninit();

    // Call the MATLAB generated function
    unsafe {
        bindings::tic_tac_toe(&input, output.as_mut_ptr());
        let output = output.assume_init();
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_move() {
        // Initialize the library
        initialize();

        // Create an empty board
        let current_state = [0u8; 9];
        let player_turn = 1; // Player 1's turn
        let proposed_move = 5; // Center position (1-indexed in MATLAB, so 5 is the center)

        // Make the move
        let TicTacToeOutput {
            was_legal,
            new_state,
            next_player_turn,
            winner,
        } = make_move(TicTacToeInput {
            current_state,
            player_turn,
            proposed_move,
        });

        // Check that the move was legal
        assert!(was_legal != 0);

        // Check that the board was updated correctly - convert to 0-indexed for checking array
        assert_eq!(new_state[(proposed_move - 1) as usize], player_turn);

        // Check that the player turn was switched
        assert_eq!(next_player_turn, 2);

        // Check that the winner is correct
        assert_eq!(winner, 0);

        // Terminate the library
        terminate();
    }
}
