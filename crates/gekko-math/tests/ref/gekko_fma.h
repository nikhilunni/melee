/*
 * PowerPC fused multiply-add forms, the C twin of crates/gekko-math/src/fma.rs.
 *
 * Operands are in instruction order (frA, frC, frB), so a line of retail
 * assembly `fmadds f1, f2, f3, f4` transcribes to gekko_fmadds(f2, f3, f4)
 * on both sides. The retail-faithful oracle sources under tests/ref/.../retail/
 * call these where the disassembly shows a fused instruction; everything
 * else in those files is compiled with -ffp-contract=off so no other
 * contraction can sneak in.
 *
 * __builtin_fma/__builtin_fmaf are recognised even under -fno-builtin and are
 * a single correctly rounded operation (hardware FMA or libm's fma), which is
 * what the Gekko computes: the single forms round the exact a*c+b to single
 * once, and with single inputs f32::mul_add / fmaf give the same bits.
 *
 * The negated forms round first and negate second, as the PowerPC ISA says
 * ("the result is rounded, then negated"), which matters for the sign of an
 * exact zero: fnmsubs(3, 2, 6) is -(6 - 6) = -0.0. Clang folds -fmaf(a, c, -b)
 * into a single host instruction that computes b - a*c = +0.0 instead, even
 * at -O0, so the intermediate is held in a volatile to keep the negation a
 * separate step. gekko_math::fma does the same negation in Rust.
 *
 * Header-only, C99, no libm dependency beyond fma/fmaf themselves.
 */
#ifndef GEKKO_MATH_REF_GEKKO_FMA_H
#define GEKKO_MATH_REF_GEKKO_FMA_H

/* fmadds frD, frA, frC, frB : frA * frC + frB */
static inline float gekko_fmadds(float a, float c, float b)
{
    return __builtin_fmaf(a, c, b);
}

/* fmsubs : frA * frC - frB */
static inline float gekko_fmsubs(float a, float c, float b)
{
    return __builtin_fmaf(a, c, -b);
}

/* fnmadds : -(frA * frC + frB) */
static inline float gekko_fnmadds(float a, float c, float b)
{
    volatile float rounded = __builtin_fmaf(a, c, b);
    return -rounded;
}

/* fnmsubs : -(frA * frC - frB) */
static inline float gekko_fnmsubs(float a, float c, float b)
{
    volatile float rounded = __builtin_fmaf(a, c, -b);
    return -rounded;
}

/* fmadd : frA * frC + frB, double */
static inline double gekko_fmadd(double a, double c, double b)
{
    return __builtin_fma(a, c, b);
}

/* fmsub : frA * frC - frB, double */
static inline double gekko_fmsub(double a, double c, double b)
{
    return __builtin_fma(a, c, -b);
}

/* fnmsub : -(frA * frC - frB), double */
static inline double gekko_fnmsub(double a, double c, double b)
{
    volatile double rounded = __builtin_fma(a, c, -b);
    return -rounded;
}

#endif
