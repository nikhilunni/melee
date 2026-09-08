/*
 * Gekko `frsqrte` / `fres` estimate model in C, the twin of
 * crates/gekko-math/src/estimate.rs, for the native reference-oracle builds
 * (gekko-math tests/ref/msl/driver.c, melee-lb tests/ref/lbtrigf/shim/math.h,
 * hsd-anim tests/ref/mtx/driver.c). Keep the two in step: the gekko-math
 * oracle test compares them directly over a large sweep.
 *
 * Tables and rules were fitted from input/output pairs captured by executing
 * the instructions in Dolphin (harness/gekko_probe); see the Rust module docs
 * for the model and the derivation. NaN inputs pass through unchanged, as on
 * the interpreter (Dolphin's ARM64 JIT would set the quiet bit).
 *
 * Header-only, C99, no libm. Every function is static so each oracle
 * translation unit gets its own copy.
 */
#ifndef GEKKO_MATH_REF_GEKKO_ESTIMATE_H
#define GEKKO_MATH_REF_GEKKO_ESTIMATE_H

#include <stdint.h>
#include <string.h>

#define GEKKO_DEFAULT_NAN 0x7FF8000000000000ULL
#define GEKKO_FLT_MAX_F64 0x47EFFFFFE0000000ULL
#define GEKKO_F64_MANT ((1ULL << 52) - 1)
#define GEKKO_F64_SIGN (1ULL << 63)

/* [parity][top 4 mantissa bits] = {base, slope}; mant = (base - slope*next11) << 26 */
static const uint32_t gekko_frsqrte_table[2][16][2] = {
    {
        {0x1A7E800, 0x0568}, {0x17CB800, 0x04F3}, {0x1552800, 0x048D}, {0x130C000, 0x0435},
        {0x10F2000, 0x03E7}, {0x0EFF000, 0x03A2}, {0x0D2E000, 0x0365}, {0x0B7C000, 0x032E},
        {0x09E5000, 0x02FC}, {0x0867000, 0x02D0}, {0x06FF000, 0x02A8}, {0x05AB800, 0x0283},
        {0x046A000, 0x0261}, {0x0339800, 0x0243}, {0x0218800, 0x0226}, {0x0105800, 0x020B},
    },
    {
        {0x3FFA000, 0x07A4}, {0x3C29000, 0x0700}, {0x38AA000, 0x0670}, {0x3572000, 0x05F2},
        {0x3279000, 0x0584}, {0x2FB7000, 0x0524}, {0x2D26000, 0x04CC}, {0x2AC0000, 0x047E},
        {0x2881000, 0x043A}, {0x2665000, 0x03FA}, {0x2468000, 0x03C2}, {0x2287000, 0x038E},
        {0x20C1000, 0x035E}, {0x1F12000, 0x0332}, {0x1D79000, 0x030A}, {0x1BF4000, 0x02E6},
    },
};

/* [top 5 mantissa bits] = {base, slope}; mant = ((base - slope*next10) >> 1) << 29 */
static const uint32_t gekko_fres_table[32][2] = {
    {0xFFF000, 0x3E1}, {0xF07000, 0x3A7}, {0xE1D400, 0x371}, {0xD41000, 0x340},
    {0xC71000, 0x313}, {0xBAC400, 0x2EA}, {0xAF2000, 0x2C4}, {0xA41000, 0x2A0},
    {0x999000, 0x27F}, {0x8F9400, 0x261}, {0x861000, 0x245}, {0x7D0000, 0x22A},
    {0x745800, 0x212}, {0x6C1000, 0x1FB}, {0x642800, 0x1E5}, {0x5C9400, 0x1D1},
    {0x555000, 0x1BE}, {0x4E5800, 0x1AC}, {0x47AC00, 0x19B}, {0x413C00, 0x18B},
    {0x3B1000, 0x17C}, {0x352000, 0x16E}, {0x2F5C00, 0x15B}, {0x29F000, 0x15B},
    {0x248800, 0x143}, {0x1F7C00, 0x143}, {0x1A7000, 0x12D}, {0x15BC00, 0x12D},
    {0x110800, 0x11A}, {0x0CA000, 0x11A}, {0x083800, 0x108}, {0x041800, 0x106},
};

static uint64_t gekko_f64_bits(double d)
{
    uint64_t u;
    memcpy(&u, &d, 8);
    return u;
}
static double gekko_bits_f64(uint64_t u)
{
    double d;
    memcpy(&d, &u, 8);
    return d;
}
static uint32_t gekko_f32_bits(float f)
{
    uint32_t u;
    memcpy(&u, &f, 4);
    return u;
}
static float gekko_bits_f32(uint32_t u)
{
    float f;
    memcpy(&f, &u, 4);
    return f;
}

static uint64_t gekko_frsqrte_bits(uint64_t bits)
{
    uint64_t sign = bits & GEKKO_F64_SIGN;
    int exp = (int) ((bits >> 52) & 0x7FF);
    uint64_t mant = bits & GEKKO_F64_MANT;
    if (exp == 0x7FF) {
        if (mant != 0) {
            return bits;
        }
        return sign ? GEKKO_DEFAULT_NAN : 0;
    }
    if (exp == 0 && mant == 0) {
        return sign | (0x7FFULL << 52);
    }
    if (sign) {
        return GEKKO_DEFAULT_NAN;
    }
    if (exp == 0) {
        int shift = 0;
        while (!((mant << shift) & (1ULL << 52))) {
            shift++;
        }
        mant = (mant << shift) & GEKKO_F64_MANT;
        exp = 1 - shift;
    }
    {
        const uint32_t* ent = gekko_frsqrte_table[exp & 1][(mant >> 48) & 0xF];
        uint32_t next11 = (uint32_t) ((mant >> 37) & 0x7FF);
        uint64_t mant_out = (uint64_t) (ent[0] - ent[1] * next11) << 26;
        uint64_t exp_out = (uint64_t) ((0xBFC - exp) >> 1);
        return (exp_out << 52) | mant_out;
    }
}

static double gekko_frsqrte(double x)
{
    return gekko_bits_f64(gekko_frsqrte_bits(gekko_f64_bits(x)));
}

static uint64_t gekko_fres_bits(uint64_t bits)
{
    uint64_t sign = bits & GEKKO_F64_SIGN;
    int exp = (int) ((bits >> 52) & 0x7FF);
    uint64_t mant = bits & GEKKO_F64_MANT;
    if (exp == 0x7FF) {
        return mant ? bits : sign;
    }
    if (exp == 0 && mant == 0) {
        return sign | (0x7FFULL << 52);
    }
    if (exp < 0x37F) {
        return sign | GEKKO_FLT_MAX_F64;
    }
    if (exp > 0x47C) {
        return sign;
    }
    {
        const uint32_t* ent = gekko_fres_table[(mant >> 47) & 0x1F];
        uint32_t next10 = (uint32_t) ((mant >> 37) & 0x3FF);
        uint64_t mant_out = (uint64_t) ((ent[0] - ent[1] * next10) >> 1) << 29;
        uint64_t exp_out = (uint64_t) (0x7FD - exp);
        return sign | (exp_out << 52) | mant_out;
    }
}

static double gekko_fres_f64(double x)
{
    return gekko_bits_f64(gekko_fres_bits(gekko_f64_bits(x)));
}

/* lfs: widen single bits to double bits (denormals normalise). */
static uint64_t gekko_widen_bits(uint32_t bits)
{
    uint64_t sign = (uint64_t) (bits >> 31) << 63;
    int exp = (int) ((bits >> 23) & 0xFF);
    uint64_t mant = bits & 0x7FFFFF;
    if (exp == 0xFF) {
        return sign | (0x7FFULL << 52) | (mant << 29);
    }
    if (exp == 0) {
        int shift = 0;
        if (mant == 0) {
            return sign;
        }
        while (!((mant << shift) & (1ULL << 23))) {
            shift++;
        }
        mant = (mant << shift) & 0x7FFFFF;
        return sign | ((uint64_t) (1 - shift + 0x380) << 52) | (mant << 29);
    }
    return sign | ((uint64_t) (exp + 0x380) << 52) | (mant << 29);
}

/* stfs of a fres result: exact narrowing (the result is single-representable). */
static uint32_t gekko_narrow_fres_bits(uint64_t bits)
{
    uint32_t sign = (uint32_t) (bits >> 63) << 31;
    int exp = (int) ((bits >> 52) & 0x7FF);
    uint32_t mant = (uint32_t) ((bits & GEKKO_F64_MANT) >> 29);
    if (exp == 0x7FF) {
        return sign | (0xFFu << 23) | mant;
    }
    if (exp == 0) {
        return sign;
    }
    return sign | ((uint32_t) (exp - 0x380) << 23) | mant;
}

/* fres on a single: lfs / fres / stfs. */
static float gekko_fres(float x)
{
    return gekko_bits_f32(gekko_narrow_fres_bits(gekko_fres_bits(gekko_widen_bits(gekko_f32_bits(x)))));
}

#endif
