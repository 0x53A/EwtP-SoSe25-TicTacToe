/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: flipud.h
 *
 */

#ifndef FLIPUD_H
#define FLIPUD_H

/* Include Files */
#include "rtwtypes.h"
#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Function Declarations */
void b_flipud(unsigned char x[9]);

void flipud(unsigned char x_data[], const int x_size[2]);

#ifdef __cplusplus
}
#endif

#endif
/*
 * File trailer for flipud.h
 *
 * [EOF]
 */
