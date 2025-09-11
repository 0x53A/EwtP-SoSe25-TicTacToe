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
            GameStage::InProgress(state)
            | GameStage::Won(_, state)
            | GameStage::Draw(state)
            | GameStage::IllegalMove(state, _) => *state,
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

        let cell_offset = |cell_index: usize| -> (usize, usize) {
            let x = (cell_index % 3) * 5 + 1; // 5 pixels per cell, 1 pixel border
            let y = (cell_index / 3) * 5 + 1;
            (x, y)
        };

        for (i, cell) in board_state.board.iter().enumerate() {
            if let Some(player) = cell {
                let (x, y) = cell_offset(i);
                draw_player(*player, (x, y));
            }
        }

        if let GameStage::IllegalMove(_, played_move) = game_stage {
            // highlight the illegal move
            let (x, y) = cell_offset(played_move as usize - 1);
            for dx in 0..3 {
                for dy in 0..3 {
                    let pixel = xy(&mut colors, x + dx, y + dy);
                    if *pixel == RGB8::new(0, 0, 0) {
                        // only highlight empty cells
                        *pixel = RGB8::new(5, 0, 0);
                    }
                }
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
            GameStage::IllegalMove(_, _) | GameStage::InProgress(_) => {
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
