/*
 * Academic License - for use in teaching, academic research, and meeting
 * course requirements at degree granting institutions only.  Not for
 * government, commercial, or other organizational use.
 * File: diag.h
 *
 */

#ifndef DIAG_H
#define DIAG_H

/* Include Files */
#include "rtwtypes.h"
#include <stddef.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Function Declarations */
int diag(const unsigned char v_data[], const int v_size[2],
         unsigned char d_data[]);

#ifdef __cplusplus
}
#endif

#endif
/*
 * File trailer for diag.h
 *
 * [EOF]
 */
