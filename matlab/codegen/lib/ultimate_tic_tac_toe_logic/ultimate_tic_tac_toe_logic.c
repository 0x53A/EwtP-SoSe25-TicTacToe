/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: ultimate_tic_tac_toe_logic.c
 *
 */

/* Include Files */
#include "ultimate_tic_tac_toe_logic.h"
#include "all.h"
#include "diag.h"
#include "flipud.h"
#include "ultimate_tic_tac_toe_logic_types.h"
#include <string.h>

/* Function Definitions */
/*
 * ULTIMATE_TIC_TAC_TOE_LOGIC Pure game logic for Ultimate Tic-Tac-Toe.
 *  Designed for code generation / C-export.
 *
 *  Input (fields, types):
 *   - current_grid_state: uint8 9x9 array (rows, cols) representing full board
 *       (0 = empty, 1 = player 1 (X), 2 = player 2 (O)). Stored as uint8.
 *   - current_grid_winners: uint8 3x3 array with winners of mini-grids
 * (0=none,1,2,3=? draw->3 or 255?) We'll use 0=none, 1=player1, 2=player2,
 * 3=draw.
 *   - player_turn: uint8 scalar (1 or 2)
 *   - proposed_move_grid: uint8 scalar 1..9 selecting the mini-grid
 * (1=row-major index)
 *   - proposed_move_cell: uint8 scalar 1..9 selecting the cell inside the
 * mini-grid (1=row-major index)
 *
 *  Output (fields, types):
 *   - was_legal: uint8 (1 legal, 0 illegal)
 *   - new_grid_state: uint8 9x9 array
 *   - new_grid_winners: uint8 3x3 array
 *   - next_player_turn: uint8 (1 or 2)
 *   - winner: uint8 (0 none, 1 or 2 overall winner)
 *   - next_grid: uint8 (0 = free choice, 1..9 = required mini-grid)
 *
 *  The board layout and indexing is row-major. Mini-grid numbering:
 *  1 2 3
 *  4 5 6
 *  7 8 9
 *
 * Arguments    : const struct0_T *input
 *                struct1_T *output
 * Return Type  : void
 */
void ultimate_tic_tac_toe_logic(const struct0_T *input, struct1_T *output)
{
  int mini_block_size[2];
  int output_size[2];
  int i;
  int i2;
  unsigned char mini_block_data[81];
  unsigned char tmp_data[81];
  unsigned char v[9];
  /*  Input validation */
  /*  Initialize outputs */
  output->winner = 0U;
  output->next_grid = 0U;
  /*  Copy state */
  memcpy(&output->new_grid_state[0], &input->current_grid_state[0],
         81U * sizeof(unsigned char));
  for (i = 0; i < 9; i++) {
    output->new_grid_winners[i] = input->current_grid_winners[i];
  }
  /*  Validate proposed grid and cell */
  if ((input->proposed_move_grid < 1) || (input->proposed_move_grid > 9) ||
      (input->proposed_move_cell < 1) || (input->proposed_move_cell > 9)) {
    /*  illegal */
    output->was_legal = 0U;
    output->next_player_turn = input->player_turn;
  } else {
    int cell_c;
    int cell_r;
    int i1;
    int mini_c;
    int mini_r;
    /*  Map proposed_move_grid (1..9) to top-left coordinates of mini-grid */
    /*  Use integer arithmetic only for code generation (int32) */
    /*  0-based */
    mini_r = (int)((unsigned int)(input->proposed_move_grid - 1) / 3U);
    /*  0..2 (int32) */
    mini_c = (input->proposed_move_grid - 3 * mini_r) - 1;
    /*  0..2 (int32) */
    /*  Map proposed_move_cell (1..9) to r,c within mini-grid */
    cell_r = (int)((unsigned int)(input->proposed_move_cell - 1) / 3U);
    /*  0..2 (int32) */
    cell_c = (input->proposed_move_cell - 3 * cell_r) - 1;
    /*  0..2 (int32) */
    /*  Compute absolute board indices (1-based) as int32 for indexing */
    /*  int32 */
    /*  Check if mini-grid already decided */
    i1 = mini_r + 3 * mini_c;
    if (input->current_grid_winners[i1] != 0) {
      /*  can't play in a decided mini-grid */
      output->was_legal = 0U;
      output->next_player_turn = input->player_turn;
    } else {
      int endR;
      /*  Check if the target cell is empty */
      endR = (mini_r * 3 + cell_r) + 9 * (mini_c * 3 + cell_c);
      if (input->current_grid_state[endR] != 0) {
        output->was_legal = 0U;
        output->next_player_turn = input->player_turn;
      } else {
        int exitg1;
        int exitg2;
        int loop_ub;
        int startC;
        int startR;
        unsigned char mini_winner;
        boolean_T output_data[9];
        boolean_T d[3];
        boolean_T guard1;
        boolean_T guard2;
        boolean_T guard3;
        /*  Make the move */
        output->new_grid_state[endR] = input->player_turn;
        output->was_legal = 1U;
        /*  Determine if this mini-grid now has a winner or draw */
        startR = mini_r * 3;
        endR = mini_r * 3;
        startC = mini_c * 3;
        mini_r = mini_c * 3;
        if (startR + 1 > endR + 3) {
          startR = 0;
          endR = 0;
        } else {
          endR += 3;
        }
        if (startC + 1 > mini_r + 3) {
          startC = 0;
          mini_r = 0;
        } else {
          mini_r += 3;
        }
        mini_c = endR - startR;
        mini_block_size[0] = mini_c;
        loop_ub = mini_r - startC;
        mini_block_size[1] = loop_ub;
        for (i = 0; i < loop_ub; i++) {
          for (i2 = 0; i2 < mini_c; i2++) {
            mini_block_data[i2 + mini_c * i] =
                output->new_grid_state[(startR + i2) + 9 * (startC + i)];
          }
        }
        /*  Returns 0=no winner,1=player1,2=player2,3=draw */
        mini_winner = 0U;
        /*  Rows */
        mini_r = 0;
        guard1 = false;
        guard2 = false;
        guard3 = false;
        do {
          exitg2 = 0;
          if (mini_r < 3) {
            endR = loop_ub - 1;
            output_size[0] = 1;
            output_size[1] = loop_ub;
            for (i = 0; i <= endR; i++) {
              output_data[i] = (output->new_grid_state[(startR + mini_r) +
                                                       9 * (startC + i)] == 1);
            }
            if (all(output_data, output_size)) {
              mini_winner = 1U;
              guard1 = true;
              exitg2 = 1;
            } else {
              output_size[0] = 1;
              for (i = 0; i <= endR; i++) {
                output_data[i] =
                    (output->new_grid_state[(startR + mini_r) +
                                            9 * (startC + i)] == 2);
              }
              if (all(output_data, output_size)) {
                mini_winner = 2U;
                guard1 = true;
                exitg2 = 1;
              } else {
                mini_r++;
                guard1 = false;
                guard2 = false;
                guard3 = false;
              }
            }
          } else {
            /*  Cols */
            mini_r = 0;
            exitg2 = 2;
          }
        } while (exitg2 == 0);
        if (exitg2 != 1) {
          do {
            exitg1 = 0;
            if (mini_r < 3) {
              endR = mini_c - 1;
              for (i = 0; i <= endR; i++) {
                output_data[i] =
                    (output->new_grid_state[(startR + i) +
                                            9 * (startC + mini_r)] == 1);
              }
              if (b_all(output_data, mini_c)) {
                mini_winner = 1U;
                guard1 = true;
                exitg1 = 1;
              } else {
                for (i = 0; i <= endR; i++) {
                  output_data[i] =
                      (output->new_grid_state[(startR + i) +
                                              9 * (startC + mini_r)] == 2);
                }
                if (b_all(output_data, mini_c)) {
                  mini_winner = 2U;
                  guard1 = true;
                  exitg1 = 1;
                } else {
                  mini_r++;
                }
              }
            } else {
              /*  Diags */
              mini_r = diag(mini_block_data, mini_block_size, v);
              for (i = 0; i < mini_r; i++) {
                output_data[i] = (v[i] == 1);
              }
              if (b_all(output_data, mini_r)) {
                guard3 = true;
              } else {
                endR = mini_c * loop_ub;
                if (endR - 1 >= 0) {
                  memcpy(&tmp_data[0], &mini_block_data[0],
                         (unsigned int)endR * sizeof(unsigned char));
                }
                flipud(tmp_data, mini_block_size);
                mini_r = diag(tmp_data, mini_block_size, v);
                for (i = 0; i < mini_r; i++) {
                  output_data[i] = (v[i] == 1);
                }
                if (b_all(output_data, mini_r)) {
                  guard3 = true;
                } else {
                  mini_r = diag(mini_block_data, mini_block_size, v);
                  for (i = 0; i < mini_r; i++) {
                    output_data[i] = (v[i] == 2);
                  }
                  if (b_all(output_data, mini_r)) {
                    guard2 = true;
                  } else {
                    flipud(mini_block_data, mini_block_size);
                    mini_r = diag(mini_block_data, mini_block_size, v);
                    for (i = 0; i < mini_r; i++) {
                      output_data[i] = (v[i] == 2);
                    }
                    if (b_all(output_data, mini_r)) {
                      guard2 = true;
                    } else {
                      boolean_T b_output_data[81];
                      /*  Draw? if all non-zero */
                      for (i = 0; i < loop_ub; i++) {
                        for (i2 = 0; i2 < mini_c; i2++) {
                          mini_block_data[i2 + mini_c * i] =
                              output->new_grid_state[(startR + i2) +
                                                     9 * (startC + i)];
                        }
                      }
                      for (i = 0; i < endR; i++) {
                        b_output_data[i] = (mini_block_data[i] != 0);
                      }
                      if (b_all(b_output_data, endR)) {
                        output->new_grid_winners[i1] = 3U;
                        /*  draw */
                      } else {
                        guard1 = true;
                      }
                    }
                  }
                }
              }
              exitg1 = 1;
            }
          } while (exitg1 == 0);
        }
        if (guard3) {
          mini_winner = 1U;
          guard1 = true;
        }
        if (guard2) {
          mini_winner = 2U;
          guard1 = true;
        }
        if (guard1 && ((mini_winner == 1) || (mini_winner == 2))) {
          output->new_grid_winners[i1] = mini_winner;
        }
        /*  Check if the overall game has a winner */
        /*  overall is 3x3 with values 0,1,2,3(draw) */
        mini_winner = 0U;
        /*  Treat draw (3) as non-player for win detection */
        endR = 0;
        do {
          exitg2 = 0;
          if (endR < 3) {
            mini_r = output->new_grid_winners[endR];
            mini_c = mini_r;
            d[0] = (mini_r == 1);
            mini_r = output->new_grid_winners[endR + 3];
            loop_ub = mini_r;
            d[1] = (mini_r == 1);
            mini_r = output->new_grid_winners[endR + 6];
            d[2] = (mini_r == 1);
            if (c_all(d)) {
              mini_winner = 1U;
              exitg2 = 1;
            } else {
              d[0] = (mini_c == 2);
              d[1] = (loop_ub == 2);
              d[2] = (mini_r == 2);
              if (c_all(d)) {
                mini_winner = 2U;
                exitg2 = 1;
              } else {
                endR++;
              }
            }
          } else {
            endR = 0;
            exitg2 = 2;
          }
        } while (exitg2 == 0);
        if (exitg2 != 1) {
          do {
            exitg1 = 0;
            if (endR < 3) {
              mini_r = output->new_grid_winners[3 * endR];
              mini_c = mini_r;
              d[0] = (mini_r == 1);
              mini_r = output->new_grid_winners[3 * endR + 1];
              loop_ub = mini_r;
              d[1] = (mini_r == 1);
              mini_r = output->new_grid_winners[3 * endR + 2];
              d[2] = (mini_r == 1);
              if (c_all(d)) {
                mini_winner = 1U;
                exitg1 = 1;
              } else {
                d[0] = (mini_c == 2);
                d[1] = (loop_ub == 2);
                d[2] = (mini_r == 2);
                if (c_all(d)) {
                  mini_winner = 2U;
                  exitg1 = 1;
                } else {
                  endR++;
                }
              }
            } else {
              d[0] = (output->new_grid_winners[0] == 1);
              d[1] = (output->new_grid_winners[4] == 1);
              d[2] = (output->new_grid_winners[8] == 1);
              if (c_all(d)) {
                mini_winner = 1U;
              } else {
                for (i = 0; i < 9; i++) {
                  v[i] = output->new_grid_winners[i];
                }
                b_flipud(v);
                d[0] = (v[0] == 1);
                d[1] = (v[4] == 1);
                d[2] = (v[8] == 1);
                if (c_all(d)) {
                  mini_winner = 1U;
                } else {
                  d[0] = (output->new_grid_winners[0] == 2);
                  d[1] = (output->new_grid_winners[4] == 2);
                  d[2] = (output->new_grid_winners[8] == 2);
                  if (c_all(d)) {
                    mini_winner = 2U;
                  } else {
                    d[0] = (v[0] == 2);
                    d[1] = (v[4] == 2);
                    d[2] = (v[8] == 2);
                    if (c_all(d)) {
                      mini_winner = 2U;
                    }
                  }
                }
              }
              exitg1 = 1;
            }
          } while (exitg1 == 0);
        }
        if ((mini_winner == 1) || (mini_winner == 2)) {
          output->winner = mini_winner;
        }
        /*  Determine next mini-grid to play in based on the cell position
         * within mini-grid */
        /*  int32 0..2 */
        /*  If that mini-grid is available (winner==0), enforce it, else free
         * choice (0) */
        if (output->new_grid_winners[cell_r + 3 * cell_c] == 0) {
          output->next_grid = (unsigned char)((cell_r * 3 + cell_c) + 1);
        } else {
          output->next_grid = 0U;
        }
        /*  Switch player */
        if (input->player_turn == 1) {
          output->next_player_turn = 2U;
        } else {
          output->next_player_turn = 1U;
        }
      }
    }
  }
}

/*
 * File trailer for ultimate_tic_tac_toe_logic.c
 *
 * [EOF]
 */
