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
 *       PowerPC `fnmsubs`: -(a * b - c), fused, rounded once. The same
 *       gekko_fnmsubs the retail/ copy uses (operands in instruction order,
 *       which is also the order the C intrinsic takes).
 *
 *   gekko_fmadds, gekko_fnmsubs, ...
 *       crates/gekko-math/tests/ref/gekko_fma.h, the C twin of
 *       gekko_math::fma, for the fused sites in retail/lbtrigf.c.
 *
 *   sqrtf(float)
 *       MSL's inline sqrtf from src/MSL/math_ppc.h on top of the __frsqrte
 *       shim above, with the retail fnmsub (or the verbatim C under
 *       -DLB_REF_UNFUSED) so it matches gekko_math::msl::sqrtf. Referenced
 *       by lb_8000D148 in lb_00CE.c, which is compiled but not exercised.
 *
 * Nothing here does arithmetic beyond those stand-ins, so a bit-for-bit
 * match between the retail/ build (with -ffp-contract=off) and the Rust
 * port shows the Rust performs the retail instruction sequence.
 */
#ifndef MELEE_LB_REF_SHIM_MATH_H
#define MELEE_LB_REF_SHIM_MATH_H

#include "../../../../../gekko-math/tests/ref/gekko_estimate.h"
#include "../../../../../gekko-math/tests/ref/gekko_fma.h"

#define M_PI 3.14159265358979323846
#define M_PI_2 (M_PI / 2)

static inline double __frsqrte(double x)
{
    return gekko_frsqrte(x);
}

static inline float __fnmsubs(float a, float b, float c)
{
    return gekko_fnmsubs(a, b, c);
}

/* One Newton step of MSL's inline sqrtf; retail sqrtf__Ff 0x8000D5D8..
 * 0x8000D5E4 is fmul, fmul, fnmsub, fmul. */
static inline double msl_sqrtf_newton_step(double x, double guess)
{
#ifdef LB_REF_UNFUSED
    return 0.5 * guess * (3.0 - guess * guess * x);
#else
    return (0.5 * guess) * gekko_fnmsub(x, guess * guess, 3.0);
#endif
}

/* MSL sqrtf (src/MSL/math_ppc.h) over the shims above. */
static inline float sqrtf(float x)
{
    if (x > 0.0f) {
        double guess = __frsqrte((double) x);
        guess = msl_sqrtf_newton_step(x, guess);
        guess = msl_sqrtf_newton_step(x, guess);
        guess = msl_sqrtf_newton_step(x, guess);
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
