/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: flipud.c
 *
 */

/* Include Files */
#include "flipud.h"

/* Function Definitions */
/*
 * Arguments    : unsigned char x[9]
 * Return Type  : void
 */
void flipud(unsigned char x[9])
{
  unsigned char xtmp;
  xtmp = x[0];
  x[0] = x[2];
  x[2] = xtmp;
  xtmp = x[3];
  x[3] = x[5];
  x[5] = xtmp;
  xtmp = x[6];
  x[6] = x[8];
  x[8] = xtmp;
}

/*
 * File trailer for flipud.c
 *
 * [EOF]
 */
