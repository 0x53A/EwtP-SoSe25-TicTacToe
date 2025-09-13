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

// Player colors
const PLAYER_1_COLOR: RGB8 = RGB8::new(0, 100, 0);
const PLAYER_2_COLOR: RGB8 = RGB8::new(0, 0, 100);

const ERROR_GLOW: RGB8 = RGB8::new(10, 0, 0); // red glow for illegal move

const CURRENT_GRID_GLOW: RGB8 = RGB8::new(5, 0, 5); // glow for current grid selection

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

        /// 0 based indizes
        let cell_offset = |board_idx: usize, cell_idx: usize| -> (usize, usize) {
            let board_column = board_idx % 3;
            let board_row = board_idx / 3;

            let x_grid = 1 + board_column * 5;
            let y_grid = 1 + board_row * 5;

            let cell_x = cell_idx % 3;
            let cell_y = cell_idx / 3;

            let x = x_grid + cell_x;
            let y = y_grid + cell_y;
            (x, y)
        };

        // Compute selection glow and selection state
        let selection = match game_stage {
            GameStage::InProgress(_, sel) => Some(sel),
            GameStage::IllegalMove(_, sel, _) => Some(sel),
            _ => None,
        };

        // Draw occupied cells (one pixel per cell)
        for i_board in 0..9 {
            for i_cell in 0..9 {
                if let Some(player) = board_state.board[i_board][i_cell] {
                    let (x, y) = cell_offset(i_board, i_cell);

                    *xy(&mut colors, x, y) = match player {
                        Player::PlayerOne => PLAYER_1_COLOR,
                        Player::PlayerTwo => PLAYER_2_COLOR,
                    };
                }
            }
        }

        let board_glow_amount: u8 = 10;
        for i_board in 0..9 {
            if let Some(finished) = board_state.finished_grids[i_board] {
                // compute winner color or white for draw
                let winner_color = match finished {
                    crate::game::PlayerOrDraw::Player(p) => match p {
                        Player::PlayerOne => PLAYER_1_COLOR,
                        Player::PlayerTwo => PLAYER_2_COLOR,
                    },
                    crate::game::PlayerOrDraw::Draw => RGB8::new(255, 255, 255),
                };

                let glow_color = RGB8::new(
                    (winner_color.r as u16 * board_glow_amount as u16 / 255) as u8,
                    (winner_color.g as u16 * board_glow_amount as u16 / 255) as u8,
                    (winner_color.b as u16 * board_glow_amount as u16 / 255) as u8,
                );

                for i_cell in 0..9 {
                    if board_state.board[i_board][i_cell].is_none() {
                        let (x, y) = cell_offset(i_board, i_cell);

                        let pixel = xy(&mut colors, x, y);
                        // add glow by mixing a small amount of glow_color into the pixel
                        *pixel = glow_color;
                    }
                }

                let (left, top) = cell_offset(i_board, 0);
                let (right, bottom) = cell_offset(i_board, 8);

                for x in left - 1..right + 2 {
                    *xy(&mut colors, x, top - 1) = glow_color;
                    *xy(&mut colors, x, bottom + 1) = glow_color;
                }

                for y in top - 1..bottom + 2 {
                    *xy(&mut colors, left - 1, y) = glow_color;
                    *xy(&mut colors, right + 1, y) = glow_color;
                }
            }
        }

        let glow_pulse: RGB8 = {
            let elapsed = (embassy_time::Instant::now() - last_changed).as_millis() as f32 / 1000.0;
            let omega = 2.0 * core::f32::consts::PI * 1.0; // 1 Hz pulse
            let env = (1.0 + libm::cosf(omega * elapsed)) * 0.5;
            RGB8::new(
                (CURRENT_GRID_GLOW.r as f32 * env) as u8,
                (CURRENT_GRID_GLOW.g as f32 * env) as u8,
                (CURRENT_GRID_GLOW.b as f32 * env) as u8,
            )
        };

        // Apply selection glow: if SelectGrid => all empty cells glow; if SelectCell(grid)
        // => only empty cells inside that big-grid glow. Do not glow border pixels.
        if let Some(sel) = selection {
            match sel {
                crate::game::NextUserSelection::SelectGrid => {
                    for i_board in 0..9 {
                        if board_state.finished_grids[i_board].is_some() {
                            // skip finished big-grids
                            continue;
                        }
                        for i_cell in 0..9 {
                            if board_state.board[i_board][i_cell].is_none() {
                                let (x, y) = cell_offset(i_board, i_cell);

                                let pixel = xy(&mut colors, x, y);
                                *pixel = RGB8::new(
                                    pixel.r.saturating_add(glow_pulse.r),
                                    pixel.g.saturating_add(glow_pulse.g),
                                    pixel.b.saturating_add(glow_pulse.b),
                                );
                            }
                        }
                    }
                }
                crate::game::NextUserSelection::SelectCell(grid) => {
                    let i_grid: usize = (grid - 1) as usize;
                    for i_cell in 0..9 {
                        if board_state.board[i_grid][i_cell].is_none() {
                            let (x, y) = cell_offset(i_grid, i_cell);

                            let pixel = xy(&mut colors, x, y);
                            *pixel = RGB8::new(
                                pixel.r.saturating_add(glow_pulse.r),
                                pixel.g.saturating_add(glow_pulse.g),
                                pixel.b.saturating_add(glow_pulse.b),
                            );
                        }
                    }
                }
            }
        }

        if let GameStage::IllegalMove(_, _, played_move) = game_stage {
            let (x, y) = cell_offset(
                (played_move.grid - 1) as usize,
                (played_move.cell - 1) as usize,
            );
            // compute a pulsing glow effect
            let pixel = xy(&mut colors, x, y);
            let elapsed = (embassy_time::Instant::now() - last_changed).as_millis() as f32 / 1000.0;
            let omega = 2.0 * core::f32::consts::PI * 2.0;
            let env = (1.0 + libm::cosf(omega * elapsed)) * 0.5;
            let blend_channel = |a: u8, b: u8, env: f32| -> u8 {
                let af = (a as f32) * env;
                let bf = (b as f32) * (1.0 - env);
                let sum = af + bf;
                if sum >= 255.0 { 255u8 } else { sum as u8 }
            };
            let new_r = blend_channel(ERROR_GLOW.r, pixel.r, env);
            let new_g = blend_channel(ERROR_GLOW.g, pixel.g, env);
            let new_b = blend_channel(ERROR_GLOW.b, pixel.b, env);

            *pixel = RGB8::new(new_r, new_g, new_b);
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
            last_changed = embassy_time::Instant::now();
        }
    }
}
