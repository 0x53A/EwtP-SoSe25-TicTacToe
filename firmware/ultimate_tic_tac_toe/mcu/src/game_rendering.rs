use alloc::boxed::Box;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use esp_println::println;
use smart_leds::RGB8;

use crate::{
    MATRIX_WIDTH,
    game::{GameStage, Player},
};

/// Convert from x,y coordinates to the linear NeoPixel index
/// The XY coordinates are 0-indexed, with (0,0) at the top-left
/// x goes right, y goes down
fn xy<T>(arr: &mut [T], x: usize, y: usize) -> &mut T {
    // the strip starts at top left, goes down, then one right and up, one right and down, ...
    // so even columns go down, odd columns go up.
    let index = if x % 2 == 0 {
        // Even columns go down
        (x * MATRIX_WIDTH) + y
    } else {
        // Odd columns go up
        (x * MATRIX_WIDTH) + (MATRIX_WIDTH - 1 - y)
    };
    &mut arr[index]
}

#[cfg(test)]
mod test_xy {
    use super::xy;

    #[test]
    fn test_xy_function() {
        let mut arr = [0; 256];
        assert!(xy(&mut arr, 0, 0) == &mut arr[0]);
        assert!(xy(&mut arr, 0, 1) == &mut arr[1]);
        assert!(xy(&mut arr, 0, 15) == &mut arr[15]);
        assert!(xy(&mut arr, 1, 15) == &mut arr[16]);
        assert!(xy(&mut arr, 1, 0) == &mut arr[31]);
    }
}

const PLAYER_1_COLOR: RGB8 = RGB8::new(0, 0, 255);
const PLAYER_2_COLOR: RGB8 = RGB8::new(255, 0, 0);

#[embassy_executor::task]
pub async fn render_task(
    input_signal: &'static Signal<CriticalSectionRawMutex, GameStage>,
    output_signal: &'static Signal<CriticalSectionRawMutex, Box<[RGB8]>>,
) -> ! {
    println!("Render task started");

    // 60 Hz refresh rate, updating the display over SPI takes around 10 ms anyway
    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(1000 / 60));

    // wait for the inital value, until then, render a spinner
    {
        let mut i = 0;

        while !input_signal.signaled() {
            // Demo: Three sine waves cycling through the 16x16 matrix
            // Red starts at 0, Blue at 1/3, Green at 2/3 of the cycle
            let mut colors = [RGB8::new(0, 0, 0); 256];

            let time_offset = (i as f32) * 0.1; // Animation speed

            for led_index in 0..256 {
                let position = (led_index as f32) / 256.0 * 2.0 * core::f32::consts::PI;

                // Three sine waves offset by 2π/3 (120 degrees)
                let red_phase = position + time_offset;
                let blue_phase = position + time_offset + 2.0 * core::f32::consts::PI / 3.0;
                let green_phase = position + time_offset + 4.0 * core::f32::consts::PI / 3.0;

                // Calculate sine values and convert to 0-255 range
                let red = ((libm::sinf(red_phase)) * 255.0) as u8;
                let green = ((libm::sinf(green_phase)) * 255.0) as u8;
                let blue = ((libm::sinf(blue_phase)) * 255.0) as u8;

                colors[led_index] = RGB8::new(red, green, blue);
            }

            output_signal.signal(Box::new(colors));

            ticker.next().await;
            i += 1;
        }
    }

    let mut game_stage: GameStage;
    game_stage = input_signal.wait().await;
    let mut last_changed = embassy_time::Instant::now();

    loop {
        let board_state = match &game_stage {
            GameStage::InProgress(state, _)
            | GameStage::Won(_, state)
            | GameStage::Draw(state)
            | GameStage::IllegalMove(state, _, _) => *state,
        };

        let mut colors = [RGB8::new(0, 0, 0); 256];

        // render board state

        let mut draw_player = |player: Player, offset: (usize, usize)| {
            let (x_offset, y_offset) = offset;
            match player {
                Player::PlayerOne => {
                    let player_color = PLAYER_1_COLOR;
                    // draw an X
                    *xy(&mut colors, x_offset, y_offset) = player_color;
                    *xy(&mut colors, x_offset + 1, y_offset + 1) = player_color;
                    *xy(&mut colors, x_offset + 2, y_offset + 2) = player_color;
                    *xy(&mut colors, x_offset, y_offset + 2) = player_color;
                    *xy(&mut colors, x_offset + 2, y_offset) = player_color;
                }
                Player::PlayerTwo => {
                    let player_color = PLAYER_2_COLOR;
                    // draw an O
                    *xy(&mut colors, x_offset + 1, y_offset) = player_color;
                    *xy(&mut colors, x_offset, y_offset + 1) = player_color;
                    *xy(&mut colors, x_offset + 2, y_offset + 1) = player_color;
                    *xy(&mut colors, x_offset + 1, y_offset + 2) = player_color;
                }
            }
        };

        // For ultimate Tic-Tac-Toe we have a 9x9 board. We'll render one pixel per cell
        // into the 16x16 matrix. The layout uses 3x3 blocks for the big boards (3 big
        // boards per row/column) with 2-pixel borders between those big boards.
        // Mapping chosen (leftmost content starts at x=1) so that big-blocks are at
        // x = 1..3, 6..8, 11..13 and borders at x=4..5 and 9..10, leaving rightmost
        // padding columns 14..15. This fits into 0..15 indexes on the 16x16 display.
        let cell_offset = |board_idx: usize, cell_idx: usize| -> (usize, usize) {
            // board_idx: 0..8 (3x3 big boards row-major)
            // cell_idx: 0..8 (3x3 cells inside a big board row-major)
            let big_row = board_idx / 3;
            let big_col = board_idx % 3;

            let inner_row = cell_idx / 3;
            let inner_col = cell_idx % 3;

            // For each big_col allocate 3 pixels for cells and 2 pixels for border.
            // Use an initial offset of 1 pixel on the left.
            let x = 1 + big_col * 5 + inner_col;
            let y = 1 + big_row * 5 + inner_row;
            (x, y)
        };

        // Compute selection glow and selection state
        let glow = RGB8::new(0, 5, 5);
        let selection = match game_stage {
            GameStage::InProgress(_, sel) => Some(sel),
            GameStage::IllegalMove(_, sel, _) => Some(sel),
            _ => None,
        };

        // Draw occupied cells (one pixel per cell)
        for r in 0..9 {
            for c in 0..9 {
                if let Some(player) = board_state.board[r][c] {
                    let board_idx = (r / 3) * 3 + (c / 3);
                    let cell_idx = (r % 3) * 3 + (c % 3);
                    let (x, y) = cell_offset(board_idx, cell_idx);
                    if x < MATRIX_WIDTH && y < MATRIX_WIDTH {
                        *xy(&mut colors, x, y) = match player {
                            Player::PlayerOne => PLAYER_1_COLOR,
                            Player::PlayerTwo => PLAYER_2_COLOR,
                        };
                    }
                }
            }
        }

        // Apply selection glow: if SelectGrid => all empty cells glow; if SelectCell(grid)
        // => only empty cells inside that big-grid glow. Do not glow border pixels.
        if let Some(sel) = selection {
            match sel {
                crate::game::NextUserSelection::SelectGrid => {
                    for r in 0..9 {
                        for c in 0..9 {
                            if board_state.board[r][c].is_none() {
                                        let board_idx = (r / 3) * 3 + (c / 3);
                                        let cell_idx = (r % 3) * 3 + (c % 3);
                                        let (x, y) = cell_offset(board_idx, cell_idx);
                                if x < MATRIX_WIDTH && y < MATRIX_WIDTH {
                                    let pixel = xy(&mut colors, x, y);
                                    *pixel = RGB8::new(pixel.r, pixel.g.saturating_add(glow.g), pixel.b.saturating_add(glow.b));
                                }
                            }
                        }
                    }
                }
                crate::game::NextUserSelection::SelectCell(grid) => {
                    // grid is 1..9 in row-major order for the 3x3 big boards
                    let grid0 = (grid - 1) as usize;
                    let big_row = grid0 / 3;
                    let big_col = grid0 % 3;
                    for inner_r in 0..3 {
                        for inner_c in 0..3 {
                            let r = big_row * 3 + inner_r;
                            let c = big_col * 3 + inner_c;
                                if board_state.board[r][c].is_none() {
                                let board_idx = (r / 3) * 3 + (c / 3);
                                let cell_idx = (r % 3) * 3 + (c % 3);
                                let (x, y) = cell_offset(board_idx, cell_idx);
                                if x < MATRIX_WIDTH && y < MATRIX_WIDTH {
                                    let pixel = xy(&mut colors, x, y);
                                    *pixel = RGB8::new(pixel.r, pixel.g.saturating_add(glow.g), pixel.b.saturating_add(glow.b));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let GameStage::IllegalMove(_, _, played_move) = game_stage {
            // highlight the illegal move: compute absolute r,c from grid and cell
            let grid0 = (played_move.grid - 1) as usize; // 0..8
            let cell0 = (played_move.cell - 1) as usize; // 0..8
            let (x, y) = cell_offset(grid0, cell0);
            if x < MATRIX_WIDTH && y < MATRIX_WIDTH {
                let pixel = xy(&mut colors, x, y);
                *pixel = RGB8::new(pixel.r, pixel.g.saturating_add(5), pixel.b);
            }
        }

        match game_stage {
            GameStage::Won(winner, _) => {
                // flash the winner's color on the border
                let border_color = match winner {
                    Player::PlayerOne => PLAYER_1_COLOR,
                    Player::PlayerTwo => PLAYER_2_COLOR,
                };
                for x in 0..16 {
                    *xy(&mut colors, x, 0) = border_color;
                    *xy(&mut colors, x, 15) = border_color;
                }
                for y in 0..16 {
                    *xy(&mut colors, 0, y) = border_color;
                    *xy(&mut colors, 15, y) = border_color;
                }
            }
            GameStage::Draw(_) => {
                // gray border for draw
                let border_color = RGB8::new(50, 50, 50);
                for y in [0, 15] {
                    for x in 0..16 {
                        *xy(&mut colors, x, y) = border_color;
                    }
                }
            }
            GameStage::IllegalMove(_, _, _) | GameStage::InProgress(_, _) => {
                // highlight current player
                if board_state.current_player == Player::PlayerOne {
                    for x in 0..MATRIX_WIDTH {
                        *xy(&mut colors, x, 0) = PLAYER_1_COLOR;
                    }
                } else {
                    for x in 0..MATRIX_WIDTH {
                        *xy(&mut colors, x, 15) = PLAYER_2_COLOR;
                    }
                }
            }
        }

        // done rendering, push it out
        output_signal.signal(Box::new(colors));

        ticker.next().await;
        if let Some(new_data) = input_signal.try_take() {
            game_stage = new_data;
        }
    }
}
