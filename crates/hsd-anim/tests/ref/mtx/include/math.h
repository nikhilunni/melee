/*
 * Host shim <math.h> for compiling the decomp's sysdolphin/baselib/mtx.c and
 * quatlib.c natively as a reference oracle (see tests/mtx_oracle.rs).
 *
 * It shadows the system <math.h> via -I so the decomp sources see MSL's
 * M_PI and prototypes. The functions declared here are *defined* in
 * driver.c (sqrtf, atan2f, asinf, acosf: see the notes there) or come from
 * gekko-math's verbatim copy of MSL trigf.c (sinf, cosf).
 */
#ifndef HSD_ANIM_REF_MTX_MATH_H
#define HSD_ANIM_REF_MTX_MATH_H

#define M_PI 3.14159265358979323846
#define M_PI_2 (M_PI / 2)

/* The decomp maps fabsf to the __fabsf intrinsic (sign-bit clear). */
#define fabsf __builtin_fabsf

float sqrtf(float);
float sinf(float);
float cosf(float);
float atan2f(float y, float x);
float asinf(float);
float acosf(float);

#endif
