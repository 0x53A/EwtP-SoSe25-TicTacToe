/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: tic_tac_toe.c
 *
 */

/* Include Files */
#include "tic_tac_toe.h"
#include "all.h"
#include "flipud.h"
#include "tic_tac_toe_types.h"

/* Function Definitions */
/*
 * TIC_TAC_TOE Updates the game state for a Tic-Tac-Toe game.
 *    input.current_state: 9-element array representing 3x3 board, row-first
 *                         (0 = empty, 1 = player 1, 2 = player 2)
 *    input.player_turn: 1 for player 1, 2 for player 2
 *    input.proposed_move: 1-based index into playing field (1-9)
 *
 *    output.was_legal: 1 if move was legal, 0 otherwise
 *    output.new_state: 9-element array of updated game state
 *    output.next_player_turn: next player's turn (1 or 2)
 *    output.winner: 0 for no winner, 1 or 2 for when player 1 or 2 have won
 *
 * Arguments    : const struct0_T *input
 *                struct1_T *output
 * Return Type  : void
 */
void tic_tac_toe(const struct0_T *input, struct1_T *output)
{
  int i;
  /*  Type validation for code generation */
  /*  Initialize output structure */
  output->was_legal = 0U;
  output->winner = 0U;
  /*  Copy current state to new state */
  for (i = 0; i < 9; i++) {
    output->new_state[i] = input->current_state[i];
  }
  /*  Determine if move is legal */
  if ((input->proposed_move >= 1) && (input->proposed_move <= 9)) {
    /*  Check if the cell is empty (0) */
    if (input->current_state[input->proposed_move - 1] == 0) {
      /*  Update the game state with player's mark */
      output->new_state[input->proposed_move - 1] = input->player_turn;
      output->was_legal = 1U;
      /*  Switch to the other player */
      if (input->player_turn == 1) {
        output->next_player_turn = 2U;
      } else {
        output->next_player_turn = 1U;
      }
    } else {
      /*  Cell already occupied - illegal move */
      output->next_player_turn = input->player_turn;
      /*  Same player tries again */
    }
  } else {
    /*  Move out of bounds - illegal move */
    output->next_player_turn = input->player_turn;
    /*  Same player tries again */
  }
  /*  If move was illegal, ensure was_legal is 0 */
  if (output->was_legal == 0) {
    output->next_player_turn = input->player_turn;
  } else {
    int b_i;
    int exitg2;
    int unnamed_idx_0;
    int unnamed_idx_1;
    int unnamed_idx_2;
    boolean_T d[3];
    /*  Check for winner, but only if the move was legal */
    /*  Check for winner in the current state */
    /*  Returns 0 for no winner, 1 for player 1, 2 for player 2 */
    /*  Convert 1D array to 2D for easier win checking */
    /*  Check rows */
    b_i = 0;
    do {
      exitg2 = 0;
      if (b_i < 3) {
        unnamed_idx_2 = output->new_state[b_i];
        unnamed_idx_0 = unnamed_idx_2;
        d[0] = (unnamed_idx_2 == 1);
        unnamed_idx_2 = output->new_state[b_i + 3];
        unnamed_idx_1 = unnamed_idx_2;
        d[1] = (unnamed_idx_2 == 1);
        unnamed_idx_2 = output->new_state[b_i + 6];
        d[2] = (unnamed_idx_2 == 1);
        if (all(d)) {
          output->winner = 1U;
          exitg2 = 1;
        } else {
          d[0] = (unnamed_idx_0 == 2);
          d[1] = (unnamed_idx_1 == 2);
          d[2] = (unnamed_idx_2 == 2);
          if (all(d)) {
            output->winner = 2U;
            exitg2 = 1;
          } else {
            b_i++;
          }
        }
      } else {
        /*  Check columns */
        b_i = 0;
        exitg2 = 2;
      }
    } while (exitg2 == 0);
    if (exitg2 != 1) {
      int exitg1;
      do {
        exitg1 = 0;
        if (b_i < 3) {
          unnamed_idx_2 = output->new_state[3 * b_i];
          unnamed_idx_0 = unnamed_idx_2;
          d[0] = (unnamed_idx_2 == 1);
          unnamed_idx_2 = output->new_state[3 * b_i + 1];
          unnamed_idx_1 = unnamed_idx_2;
          d[1] = (unnamed_idx_2 == 1);
          unnamed_idx_2 = output->new_state[3 * b_i + 2];
          d[2] = (unnamed_idx_2 == 1);
          if (all(d)) {
            output->winner = 1U;
            exitg1 = 1;
          } else {
            d[0] = (unnamed_idx_0 == 2);
            d[1] = (unnamed_idx_1 == 2);
            d[2] = (unnamed_idx_2 == 2);
            if (all(d)) {
              output->winner = 2U;
              exitg1 = 1;
            } else {
              b_i++;
            }
          }
        } else {
          /*  Check diagonals */
          d[0] = (output->new_state[0] == 1);
          d[1] = (output->new_state[4] == 1);
          d[2] = (output->new_state[8] == 1);
          if (all(d)) {
            output->winner = 1U;
          } else {
            unsigned char v[9];
            for (i = 0; i < 9; i++) {
              v[i] = output->new_state[i];
            }
            flipud(v);
            d[0] = (v[0] == 1);
            d[1] = (v[4] == 1);
            d[2] = (v[8] == 1);
            if (all(d)) {
              output->winner = 1U;
            } else {
              d[0] = (output->new_state[0] == 2);
              d[1] = (output->new_state[4] == 2);
              d[2] = (output->new_state[8] == 2);
              if (all(d)) {
                output->winner = 2U;
              } else {
                d[0] = (v[0] == 2);
                d[1] = (v[4] == 2);
                d[2] = (v[8] == 2);
                if (all(d)) {
                  output->winner = 2U;
                } else {
                  /*  No winner */
                  output->winner = 0U;
                }
              }
            }
          }
          exitg1 = 1;
        }
      } while (exitg1 == 0);
    }
  }
}

/*
 * File trailer for tic_tac_toe.c
 *
 * [EOF]
 */
