/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: ultimate_tic_tac_toe_logic_types.h
 *
 */

#ifndef ULTIMATE_TIC_TAC_TOE_LOGIC_TYPES_H
#define ULTIMATE_TIC_TAC_TOE_LOGIC_TYPES_H

/* Include Files */
#include "rtwtypes.h"

/* Type Definitions */
#ifndef typedef_struct0_T
#define typedef_struct0_T
typedef struct {
  unsigned char current_grid_state[81];
  unsigned char current_grid_winners[9];
  unsigned char player_turn;
  unsigned char proposed_move_grid;
  unsigned char proposed_move_cell;
} struct0_T;
#endif /* typedef_struct0_T */

#ifndef typedef_struct1_T
#define typedef_struct1_T
typedef struct {
  unsigned char was_legal;
  unsigned char new_grid_state[81];
  unsigned char new_grid_winners[9];
  unsigned char next_player_turn;
  unsigned char winner;
  unsigned char next_grid;
} struct1_T;
#endif /* typedef_struct1_T */

#endif
/*
 * File trailer for ultimate_tic_tac_toe_logic_types.h
 *
 * [EOF]
 */
