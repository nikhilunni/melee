/*
 * Host shim for building the MSL math sources natively as a reference oracle.
 *
 * The four .c files next to this header (trigf.c, math_data.c, math_1.c,
 * math.c) are byte-for-byte copies of third_party/melee-decomp/src/MSL/ at
 * the commit the gekko-math port was written against; tests/ref_oracle.rs
 * checks they still match the submodule. They `#include "math.h"`, which
 * resolves to this file instead of the decomp's math.h (that one pulls in
 * Runtime/platform.h and the MetroTRK intrinsics, neither of which builds on
 * a host compiler).
 *
 * This shim provides only what those four files need:
 *   - the fixed-width typedefs from Runtime/platform.h
 *   - M_PI, SECTION_CTORS
 *   - MSL_HI / MSL_LO, adjusted for host endianness (the decomp's definition
 *     assumes big-endian: HI is the *first* int of a double)
 *   - fabsf -> the compiler builtin, mirroring `#define fabsf __fabsf`
 *   - the inline fmodf from the decomp's math.h, in two spellings: the
 *     retail one (fnmsubs, and the long long -> float conversion rounding
 *     through double like __cvt_sll_flt) by default, or the verbatim C
 *     under -DMSL_REF_UNFUSED
 *   - the gekko_fmadds/... helpers (../gekko_fma.h) the retail/ copies use
 *
 * Nothing here does arithmetic of its own beyond those stand-ins, so a
 * bit-for-bit match between the retail/ build (with -ffp-contract=off) and
 * the Rust port shows the Rust performs the retail instruction sequence.
 */
#ifndef GEKKO_MATH_REF_MSL_MATH_H
#define GEKKO_MATH_REF_MSL_MATH_H

typedef float f32;
typedef double f64;
typedef int s32;
typedef unsigned int u32;
typedef long long s64;
typedef unsigned long long u64;

#define M_PI 3.14159265358979323846
#define M_PI_2 (M_PI / 2)

#define SECTION_CTORS

#include "../gekko_fma.h"

#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
#define MSL_HI(x) *(int*) &x
#define MSL_LO(x) *(1 + (int*) &x)
#else
#define MSL_HI(x) *(1 + (int*) &x)
#define MSL_LO(x) *(int*) &x
#endif

#define fabsf __builtin_fabsf

float fabsf__Ff(float);
float sinf(float);
float cosf(float);
float tanf(float);
float sin__Ff(float x);
float cos__Ff(float x);
float logf(float);
double frexp(double x, int* exponent);
void __sinit_trigf_c(void);

/* From third_party/melee-decomp/src/MSL/math.h; the return is the audited
 * site (fmodf is inlined into its callers, aobj.c 0x80364380 and bytecode.c
 * 0x80380FE4 both end in `fnmsubs b, quotient, a`). */
static inline float fmodf(float a, float b)
{
    long long quotient;

    if (fabsf(b) > fabsf(a)) {
        return a;
    }
    quotient = a / b;
#ifdef MSL_REF_UNFUSED
    return a - b * quotient;
#else
    /* __cvt_sll_flt rounds to double, then frsp to single. */
    return gekko_fnmsubs(b, (float) (double) quotient, a);
#endif
}

#endif
