use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use matlab_code::{UltimateInput, UltimateOutput, run_ultimate, initialize};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Player {
    PlayerOne = 1,
    PlayerTwo = 2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerOrDraw {
    Player(Player),
    Draw,
}

impl Player {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Player::PlayerOne),
            2 => Some(Player::PlayerTwo),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BoardState {
    pub board: [[Option<Player>; 9]; 9],
    pub finished_grids: [Option<PlayerOrDraw>; 9],
    pub current_player: Player,
}

/// what the user is currently selecting
#[derive(Copy, Clone, Debug)]
pub enum NextUserSelection {
    /// the user must first select a mini-grid (1..9)
    SelectGrid,
    /// a mini-grid is selected, user must select a cell (1..9)
    SelectCell(/*grid*/u8),
}

#[derive(Copy, Clone, Debug)]
pub struct Move {
    pub grid: u8, // 1..9
    pub cell: u8, // 1..9
}

#[derive(Copy, Clone, Debug)]
pub enum GameStage {
    InProgress(BoardState, NextUserSelection),
    /// same as InProgress, but the last move was illegal
    IllegalMove(BoardState, NextUserSelection, /*previous_move_attempt*/Move),
    Won(Player, BoardState),
    Draw(BoardState),
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            board: [[None; 9]; 9],
            current_player: Player::PlayerOne,
            finished_grids: [None; 9],
        }
    }

    // Convert board to u8 array for MATLAB code (flattened 9x9)
    pub fn board_as_u8_array(&self) -> [u8; 81] {
        // MATLAB uses column-major ordering. The generated C code (from MATLAB)
        // expects the 9x9 array flattened such that element (r,c) maps to
        // index = r + c*9 where r and c are 0-based.
        let mut result = [0u8; 81];
        for r in 0..9 {
            for c in 0..9 {
                let flat = r + c * 9;
                result[flat] = self.board[r][c].map(|p| p as u8).unwrap_or(0);
            }
        }
        result
    }

    pub fn make_move(self, proposed_grid: u8, proposed_cell: u8) -> GameStage {
        // Prepare input for MATLAB generated function
        // Build current_grid_winners from our finished_grids in column-major order
        let mut cg_winners = [0u8; 9];
        for r in 0..3 {
            for c in 0..3 {
                let idx = r + c * 3; // column-major
                cg_winners[idx] = match self.finished_grids[idx] {
                    None => 0u8,
                    Some(PlayerOrDraw::Player(p)) => p as u8,
                    Some(PlayerOrDraw::Draw) => 3u8,
                };
            }
        }

        let input = UltimateInput {
            current_grid_state: self.board_as_u8_array(),
            current_grid_winners: cg_winners,
            player_turn: self.current_player as u8,
            proposed_move_grid: proposed_grid,
            proposed_move_cell: proposed_cell,
        };

        let UltimateOutput {
            was_legal,
            new_grid_state,
            new_grid_winners,
            next_player_turn,
            winner,
            next_grid,
        } = run_ultimate(input);

        // Map new_grid_state (column-major flattened 81 bytes) back into BoardState.board
        let mut new_board = [[None; 9]; 9];
        for r in 0..9 {
            for c in 0..9 {
                let flat = r + c * 9;
                new_board[r][c] = Player::from_u8(new_grid_state[flat]);
            }
        }

        // Map new_grid_winners (3x3 column-major) back into finished_grids
        let mut new_finished = [None; 9];
        for r in 0..3 {
            for c in 0..3 {
                let idx = r + c * 3; // column-major index
                new_finished[idx] = match new_grid_winners[idx] {
                    0 => None,
                    1 => Some(PlayerOrDraw::Player(Player::PlayerOne)),
                    2 => Some(PlayerOrDraw::Player(Player::PlayerTwo)),
                    3 => Some(PlayerOrDraw::Draw),
                    _ => None,
                };
            }
        }

        let next_selection = if next_grid == 0 {
            NextUserSelection::SelectGrid
        } else {
            NextUserSelection::SelectCell(next_grid)
        };

        let new_state = BoardState {
            board: new_board,
            current_player: Player::from_u8(next_player_turn).unwrap_or(Player::PlayerOne),
            finished_grids: new_finished,
        };

        if was_legal != 0 {
            if winner != 0 {
                let winner = Player::from_u8(winner).unwrap();
                GameStage::Won(winner, new_state)
            } else if new_state.is_draw() {
                GameStage::Draw(new_state)
            } else {
                GameStage::InProgress(new_state, next_selection)
            }
        } else {
            GameStage::IllegalMove(new_state, next_selection, Move { grid: proposed_grid, cell: proposed_cell })
        }
    }
}

impl BoardState {
    pub fn is_draw(&self) -> bool {
        // it's a draw if all 81 cells are filled
        for r in 0..9 {
            for c in 0..9 {
                if self.board[r][c].is_none() {
                    return false;
                }
            }
        }
        true
        // (this could be optimized to declare a draw earlier.)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyboardInput {
    Numpad(u8),
    Number(u8),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
}

#[embassy_executor::task]
pub async fn game_loop(
    input: &'static Signal<CriticalSectionRawMutex, KeyboardInput>,
    output: &'static Signal<CriticalSectionRawMutex, GameStage>,
) {
    // Initialize MATLAB code bindings
    initialize();

    // start at board 1 (top left)
    let mut game_stage = GameStage::InProgress(BoardState::new(), NextUserSelection::SelectCell(1));
    output.signal(game_stage);

    loop {
        let input = input.wait().await;

        match &game_stage {
            GameStage::Won(_, _) | GameStage::Draw(_) => {
                // after a game, wait for enter to create a new game
                if input == KeyboardInput::Enter {
                    game_stage = GameStage::InProgress(BoardState::new(), NextUserSelection::SelectCell(1));
                    output.signal(game_stage);
                }
                continue;
            }
            GameStage::InProgress(board_state, selection) | GameStage::IllegalMove(board_state, selection, _) => {
                match input {
                    KeyboardInput::Numpad(n) if (1..=9).contains(&n) => {
                        // Map numpad numbering to row-major 1..9 ordering used in MATLAB
                        let mapped = match n {
                            1 => 7u8,
                            2 => 8u8,
                            3 => 9u8,
                            4 => 4u8,
                            5 => 5u8,
                            6 => 6u8,
                            7 => 1u8,
                            8 => 2u8,
                            9 => 3u8,
                            _ => unreachable!(),
                        };


                        match selection {
                            NextUserSelection::SelectGrid => {
                                // first press selects the mini-grid (1..9)
                                game_stage = GameStage::InProgress(*board_state, NextUserSelection::SelectCell(mapped));
                                output.signal(game_stage);
                            }
                            NextUserSelection::SelectCell(grid) => {
                                // second press selects cell within mini-grid
                                let cell = mapped;

                                // perform move: grid and cell are both 1..9
                                game_stage = board_state.make_move(*grid, cell);
                                output.signal(game_stage);
                            }
                        }
                    }
                    _ => continue, // Ignore other keys
                }
            }
        }
    }
}
