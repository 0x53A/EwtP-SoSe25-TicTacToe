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
void b_flipud(unsigned char x[9])
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
 * Arguments    : unsigned char x_data[]
 *                const int x_size[2]
 * Return Type  : void
 */
void flipud(unsigned char x_data[], const int x_size[2])
{
  int b_i;
  int i;
  int j;
  int m;
  int md2;
  m = x_size[0] - 1;
  i = x_size[1];
  md2 = x_size[0] >> 1;
  for (j = 0; j < i; j++) {
    for (b_i = 0; b_i < md2; b_i++) {
      int b_xtmp_tmp;
      int xtmp_tmp;
      unsigned char xtmp;
      xtmp_tmp = x_size[0] * j;
      b_xtmp_tmp = b_i + xtmp_tmp;
      xtmp = x_data[b_xtmp_tmp];
      xtmp_tmp += m - b_i;
      x_data[b_xtmp_tmp] = x_data[xtmp_tmp];
      x_data[xtmp_tmp] = xtmp;
    }
  }
}

/*
 * File trailer for flipud.c
 *
 * [EOF]
 */
