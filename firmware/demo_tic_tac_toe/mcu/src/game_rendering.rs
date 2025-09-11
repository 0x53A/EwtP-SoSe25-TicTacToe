use crate::MATRIX_WIDTH;



// Convert from x,y coordinates to the linear NeoPixel index
// Adjust this function based on your specific NeoPixel layout
fn xy_to_neopixel_index(x: usize, y: usize) -> usize {
    if y % 2 == 0 {
        // Even rows go left to right
        y * MATRIX_WIDTH + x
    } else {
        // Odd rows go right to left (zigzag pattern)
        y * MATRIX_WIDTH + (MATRIX_WIDTH - 1 - x)
    }
}
