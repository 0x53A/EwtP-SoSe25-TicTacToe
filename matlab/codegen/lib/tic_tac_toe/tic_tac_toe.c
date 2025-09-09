/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: tic_tac_toe.c
 *
 * MATLAB Coder version            : 25.1
 * C/C++ source code generated on  : 09-Sep-2025 23:57:25
 */

/* Include Files */
#include "tic_tac_toe.h"
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
  }
}

/*
 * File trailer for tic_tac_toe.c
 *
 * [EOF]
 */
