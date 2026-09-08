/*
 * Host shim for building the two lb sources natively as a reference oracle.
 *
 * lbtrigf.c and lb_00CE.c `#include <math.h>`; with `-I shim` on the
 * command line this file is found first and the host libm header is never
 * seen. That matters: lbtrigf.c does `#ifndef NAN` to pick MSL's NaN datum
 * (bit pattern 0x7FFFFFFF, from src/MSL/float.c), so this file must NOT
 * define NAN, and the sources define atanf/asinf/acosf/atan2f/expf/powf
 * themselves, so no libm prototypes may be in scope.
 *
 * Stand-ins for the MetroTRK intrinsics:
 *
 *   __frsqrte(double)
 *       Gekko reciprocal-square-root estimate: the table model from
 *       crates/gekko-math/tests/ref/gekko_estimate.h, the C twin of
 *       gekko_math::estimate::frsqrte (fitted from captured hardware pairs;
 *       see that module's docs).
 *
 *   __fnmsubs(a, b, c)
 *       PowerPC `fnmsubs`: -(a * b - c), fused, rounded once. Mirrors
 *       gekko_math::fma::fnmsubs (whose operand order is (a, c, b) but whose
 *       call sites in trigf.rs pass the same three values in the same
 *       positions as the C).
 *
 *   sqrtf(float)
 *       MSL's inline sqrtf from src/MSL/math_ppc.h on top of the __frsqrte
 *       shim above, so it matches gekko_math::msl::sqrtf. Referenced by
 *       lb_8000D148 in lb_00CE.c, which is compiled but not exercised.
 *
 * Nothing here does arithmetic beyond those stand-ins, so a bit-for-bit
 * match between this build (with -ffp-contract=off) and the Rust port shows
 * the Rust is a faithful transcription of the C. It says nothing about which
 * operations MWCC fused on the real hardware; that needs the retail asm.
 */
#ifndef MELEE_LB_REF_SHIM_MATH_H
#define MELEE_LB_REF_SHIM_MATH_H

#include "../../../../../gekko-math/tests/ref/gekko_estimate.h"

#define M_PI 3.14159265358979323846
#define M_PI_2 (M_PI / 2)

static inline double __frsqrte(double x)
{
    return gekko_frsqrte(x);
}

static inline float __fnmsubs(float a, float b, float c)
{
    return -__builtin_fmaf(a, b, -c);
}

/* MSL sqrtf (src/MSL/math_ppc.h), verbatim apart from the volatile. */
static inline float sqrtf(float x)
{
    if (x > 0.0f) {
        double guess = __frsqrte((double) x);
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        guess = 0.5 * guess * (3.0 - guess * guess * x);
        return (float) (x * guess);
    }
    return x;
}

float atan2f(float y, float x);
float acosf(float x);
float asinf(float x);
float atanf(float x);
f32 expf(f32 arg8);
f32 powf(f32 arg0, f32 arg1);

#endif
