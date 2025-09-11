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

#[derive(Copy, Clone, Debug)]
pub struct BoardState {
    pub board: [Option<Player>; 9],
    pub current_player: Player,
}

#[derive(Copy, Clone, Debug)]
pub enum GameStage {
    InProgress(BoardState),
    IllegalMove(BoardState, u8),
    Won(Player, BoardState),
    Draw(BoardState),
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
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

    pub fn make_move(self, game_move: u8) -> GameStage {
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

        let new_state = BoardState {
            board: new_state.map(Player::from_u8).to_vec().try_into().unwrap(),
            current_player: Player::from_u8(next_player_turn).unwrap(),
        };

        if was_legal != 0 {
            if winner != 0 {
                let winner = Player::from_u8(winner).unwrap();
                GameStage::Won(winner, new_state)
            } else {
                GameStage::InProgress(new_state)
            }
        } else {
            GameStage::IllegalMove(new_state, game_move)
        }
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
    matlab_code::initialize();

    let mut game_stage = GameStage::InProgress(BoardState::new());
    output.signal(game_stage);

    loop {
        let input = input.wait().await;

        match &game_stage {
            GameStage::Won(_, _) | GameStage::Draw(_) => {
                // after a game, wait for enter to create a new game
                if input == KeyboardInput::Enter {
                    game_stage = GameStage::InProgress(BoardState::new());
                    output.signal(game_stage);
                }
                continue;
            }
            GameStage::InProgress(board_state) | GameStage::IllegalMove(board_state, _) => {
                let game_move = match input {
                    KeyboardInput::Numpad(n) if (1..=9).contains(&n) => {
                        // in game logic, 1 is top-left, 2 is top-middle, ..., 9 is bottom-right
                        // in numpad, 1 is bottom-left, 2 is bottom-middle, ...,
                        match n {
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
                        }
                    }
                    _ => continue, // Ignore other keys
                };

                // make the move
                game_stage = board_state.make_move(game_move);

                // update the neopixel matrix
                output.signal(game_stage);
            }
        }
    }
}
