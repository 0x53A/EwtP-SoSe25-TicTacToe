/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: all.c
 *
 */

/* Include Files */
#include "all.h"

/* Function Definitions */
/*
 * Arguments    : const boolean_T x_data[]
 *                const int x_size[2]
 * Return Type  : boolean_T
 */
boolean_T all(const boolean_T x_data[], const int x_size[2])
{
  int ix;
  boolean_T exitg1;
  boolean_T y;
  y = true;
  ix = 1;
  exitg1 = false;
  while ((!exitg1) && (ix <= x_size[1])) {
    if (!x_data[ix - 1]) {
      y = false;
      exitg1 = true;
    } else {
      ix++;
    }
  }
  return y;
}

/*
 * Arguments    : const boolean_T x_data[]
 *                int x_size
 * Return Type  : boolean_T
 */
boolean_T b_all(const boolean_T x_data[], int x_size)
{
  int ix;
  boolean_T exitg1;
  boolean_T y;
  y = true;
  ix = 1;
  exitg1 = false;
  while ((!exitg1) && (ix <= x_size)) {
    if (!x_data[ix - 1]) {
      y = false;
      exitg1 = true;
    } else {
      ix++;
    }
  }
  return y;
}

/*
 * Arguments    : const boolean_T x[3]
 * Return Type  : boolean_T
 */
boolean_T c_all(const boolean_T x[3])
{
  int k;
  boolean_T exitg1;
  boolean_T y;
  y = true;
  k = 0;
  exitg1 = false;
  while ((!exitg1) && (k < 3)) {
    if (!x[k]) {
      y = false;
      exitg1 = true;
    } else {
      k++;
    }
  }
  return y;
}

/*
 * File trailer for all.c
 *
 * [EOF]
 */
