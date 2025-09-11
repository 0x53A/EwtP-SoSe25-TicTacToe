/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: tic_tac_toe_types.h
 *
 */

#ifndef TIC_TAC_TOE_TYPES_H
#define TIC_TAC_TOE_TYPES_H

/* Include Files */
#include "rtwtypes.h"

/* Type Definitions */
#ifndef typedef_struct0_T
#define typedef_struct0_T
typedef struct {
  unsigned char current_state[9];
  unsigned char player_turn;
  unsigned char proposed_move;
} struct0_T;
#endif /* typedef_struct0_T */

#ifndef typedef_struct1_T
#define typedef_struct1_T
typedef struct {
  unsigned char was_legal;
  unsigned char new_state[9];
  unsigned char next_player_turn;
  unsigned char winner;
} struct1_T;
#endif /* typedef_struct1_T */

#endif
/*
 * File trailer for tic_tac_toe_types.h
 *
 * [EOF]
 */
