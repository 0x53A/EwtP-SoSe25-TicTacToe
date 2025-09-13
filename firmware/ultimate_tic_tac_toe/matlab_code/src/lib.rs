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

/// Initialize the Ultimate Tic-Tac-Toe library
pub fn initialize() {
    unsafe { bindings::ultimate_tic_tac_toe_logic_initialize() };
}

/// Terminate the Ultimate Tic-Tac-Toe library
pub fn terminate() {
    unsafe { bindings::ultimate_tic_tac_toe_logic_terminate() };
}

pub type UltimateInput = bindings::struct0_T;
pub type UltimateOutput = bindings::struct1_T;

/// Call the MATLAB generated ultimate game function
pub fn run_ultimate(input: UltimateInput) -> UltimateOutput {
    let mut output = MaybeUninit::<UltimateOutput>::uninit();
    unsafe {
        bindings::ultimate_tic_tac_toe_logic(&input, output.as_mut_ptr());
        output.assume_init()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_terminate() {
        initialize();

        let input = UltimateInput {
            current_grid_state: [0u8; 81],
            current_grid_winners: [0u8; 9],
            player_turn: 1,
            proposed_move_grid: 0,
            proposed_move_cell: 0,
        };
        let _output = run_ultimate(input);

        terminate();
    }
}
