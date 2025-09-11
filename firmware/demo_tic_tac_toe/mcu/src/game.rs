use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use matlab_code::{TicTacToeInput, TicTacToeOutput};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Player {
    PlayerOne = 1,
    PlayerTwo = 2,
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

#[derive(Clone, Debug)]
pub struct GameState {
    pub board: [Option<Player>; 9],
    pub current_player: Player,
}

pub enum MoveResult {
    IllegalMove(GameState),
    Won(Player, GameState),
    Updated(GameState),
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            board: [None; 9],
            current_player: Player::PlayerOne,
        }
    }

    // Convert board to u8 array for MATLAB code
    pub fn board_as_u8_array(&self) -> [u8; 9] {
        let mut result = [0u8; 9];
        for i in 0..9 {
            result[i] = self.board[i].map(|p| p as u8).unwrap_or(0);
        }
        result
    }

    pub fn make_move(self, game_move: u8) -> MoveResult {
        let TicTacToeOutput {
            was_legal,
            new_state,
            next_player_turn,
            winner,
        } = matlab_code::make_move(TicTacToeInput {
            current_state: self.board_as_u8_array(),
            player_turn: self.current_player as u8,
            proposed_move: game_move,
        });

        let new_state = GameState {
            board: new_state.map(|v| Player::from_u8(v)).to_vec().try_into().unwrap(),
            current_player: Player::from_u8(next_player_turn).unwrap(),
        };

        if was_legal != 0 {
            if winner != 0 {
                let winner = Player::from_u8(winner).unwrap();
                MoveResult::Won(winner, new_state)
            } else {
                MoveResult::Updated(new_state)
            }
        } else {
            MoveResult::IllegalMove(new_state)
        }
    }
}

pub async fn game_loop(
    input: &'static Signal<CriticalSectionRawMutex, u8>,
    output: &'static Signal<CriticalSectionRawMutex, GameState>,
) {
    matlab_code::initialize();

    let mut game_state = GameState::new();
    loop {
        let game_move = input.wait().await;
        let result = game_state.make_move(game_move);
        match &result {
            MoveResult::Updated(state)
            | MoveResult::Won(_, state)
            | MoveResult::IllegalMove(state) => {
                game_state = state.clone();
            }
        }
        output.signal(game_state.clone());
    }
}
