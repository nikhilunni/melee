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
 *   - the inline fmodf, copied verbatim from the decomp's math.h
 *
 * Nothing here does arithmetic of its own, so a bit-for-bit match between
 * this build (with -ffp-contract=off) and the Rust port shows the Rust is a
 * faithful transcription of the C. It says nothing about which operations
 * MWCC fused on the real hardware; that needs the retail asm.
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

/* Verbatim from third_party/melee-decomp/src/MSL/math.h. */
static inline float fmodf(float a, float b)
{
    long long quotient;

    if (fabsf(b) > fabsf(a)) {
        return a;
    }
    quotient = a / b;
    return a - b * quotient;
}

#endif
