/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: diag.c
 *
 */

/* Include Files */
#include "diag.h"

/* Function Definitions */
/*
 * Arguments    : const unsigned char v_data[]
 *                const int v_size[2]
 *                unsigned char d_data[]
 * Return Type  : int
 */
int diag(const unsigned char v_data[], const int v_size[2],
         unsigned char d_data[])
{
  int d_size;
  int k;
  if ((v_size[0] == 1) && (v_size[1] == 1)) {
    d_size = 1;
    d_data[0] = v_data[0];
  } else {
    int u0;
    u0 = v_size[0];
    d_size = v_size[1];
    if (u0 <= d_size) {
      d_size = u0;
    }
    if (v_size[1] <= 0) {
      d_size = 0;
    }
    for (k = 0; k < d_size; k++) {
      d_data[k] = v_data[k + v_size[0] * k];
    }
  }
  return d_size;
}

/*
 * File trailer for diag.c
 *
 * [EOF]
 */
