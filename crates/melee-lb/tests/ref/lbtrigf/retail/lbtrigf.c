/*
 * RETAIL-FAITHFUL COPY of ../lbtrigf.c (itself a verbatim copy of
 * third_party/melee-decomp/src/melee/lb/lbtrigf.c). Not verbatim: every
 * place the retail disassembly of main.dol shows a fused multiply-add is
 * rewritten with the gekko_fmadds/gekko_fnmsubs helpers from
 * crates/gekko-math/tests/ref/gekko_fma.h (included through the shim
 * math.h), each tagged with the instruction address. Everything else is
 * untouched; `diff ../lbtrigf.c lbtrigf.c` shows exactly the audited sites.
 *
 * This is what tests/ref_oracle.rs compares the Rust against. The verbatim
 * copy stays alongside so the diff, and the count of inputs on which the
 * two builds disagree, remain reviewable. lb_00CE.c (expf, powf) has no
 * fused instruction in retail and needs no copy.
 */
#include "lbtrigf.h"

#include <Runtime/platform.h>

#include <math.h>
#include <placeholder.h>

/* 022DF8 */ static float lb_sqrtf(float x);
/* 400770 */ extern float MSL_TrigF_80400770[];
/* 400774 */ extern float MSL_TrigF_80400774[];

#define SIGN_BIT (1 << 31)
#define BITWISE(f) (*(u32*) &f)
#define SIGNED_BITWISE(f) ((s32) BITWISE(f))
#define GET_SIGN_BIT(f) (SIGNED_BITWISE(f) & SIGN_BIT)
#define BITWISE_PI_2 0x3FC90FDB
#ifndef NAN
#define NAN MSL_TrigF_80400770[0]
#endif
#define INF MSL_TrigF_80400774[0]

float atan2f(float y, float x)
{
    if (GET_SIGN_BIT(x) == GET_SIGN_BIT(y)) {
        if (GET_SIGN_BIT(x) != 0) {
            return x == -0.0f ? (float) -M_PI_2 : atanf(y / x) - (float) M_PI;
        }

        return x ? atanf(y / x) : (float) M_PI_2;
    }

    if (x < 0.0f) {
        return (float) M_PI + atanf(y / x);
    }

    if (x) {
        return atanf(y / x);
    }

    *(u32*) &y = GET_SIGN_BIT(y) + BITWISE_PI_2;

    return y;
}

float acosf(float x)
{
    /* retail 0x80022D30: fnmsubs x, x, 1.0 */
    float result = gekko_fnmsubs(x, x, 1.0F);
    if (result > 0) {
        float guess;
        guess = __frsqrte(result);
        /* retail 0x80022D54, 0x80022D64, 0x80022D74: fmuls g*g, fmuls 0.5*g,
         * fnmsubs result, g*g, 3.0, fmuls; three times */
        guess = (0.5f * guess) * gekko_fnmsubs(result, guess * guess, 3.0f);
        guess = (0.5f * guess) * gekko_fnmsubs(result, guess * guess, 3.0f);
        guess = (0.5f * guess) * gekko_fnmsubs(result, guess * guess, 3.0f);
        result = guess;
    } else if (result) {
        result = NAN;
    } else {
        result = INF;
    }
    return (float) M_PI_2 - atanf(x * result);
}

float asinf(float x)
{
    /* retail 0x80022DD4: fnmsubs x, x, 1.0 */
    return atanf(x * lb_sqrtf(gekko_fnmsubs(x, x, 1.0f)));
}

static float lb_sqrtf(float x)
{
    if (x > 0.0f) {
        float guess;
        guess = __frsqrte(x);
        /* retail 0x80022E1C, 0x80022E2C, 0x80022E3C: fmuls g*g, fmuls 0.5*g,
         * fnmsubs x, g*g, 3.0, fmuls; three times */
        guess = (0.5f * guess) * gekko_fnmsubs(x, guess * guess, 3.0f);
        guess = (0.5f * guess) * gekko_fnmsubs(x, guess * guess, 3.0f);
        guess = (0.5f * guess) * gekko_fnmsubs(x, guess * guess, 3.0f);
        return guess;
    }

    if (x) {
        return NAN;
    }

    return INF;
}

#define SILVER_RATIO_1_CONJUGATE lbRefract3_804D7DD4

#define BITWISE_INF 0x7F800000 /* = +Infinity */
#define BITWISE_0_5 0x3F000000 /* = 0.5f */
#define BITWISE_1_0 0x3F800000 /* = 1.0f */
#define BITWISE_2_0 0x40000000 /* = 2.0f */

#define BITWISE_THRESHOLD_0 0x3F08D5B9 /* = 0.534511148929596f */
#define BITWISE_THRESHOLD_1 0x3F521801 /* = 0.8206787705421448f */
#define BITWISE_THRESHOLD_2 0x3F9BF7EC /* = 1.218503475189209f */
#define BITWISE_THRESHOLD_3 0x3FEF789E /* = 1.870868444442749f */

static const float atanf_lookup[] = {
    1.0,
    -0.3333333134651184,
    0.1999988704919815,
    -0.14281649887561798,
    0.11041180044412613,
    -0.08459755778312683,
    0.04714243486523628,
    6.828420162200928,
    3.239828109741211,
    2.0,
    1.4464620351791382,
    1.1715729236602783,
    1.039566159248352,
    7.1350000325764995e-06,
    8.200000252145401e-07,
    0.0,
    6.299999881775875e-07,
    0.0,
    0.0,
    0.0,
    0.3926900029182434,
    0.5890486240386963,
    0.7853981256484985,
    0.9817469716072083,
    1.1780970096588135,
    1.3744460344314575,
    0.0,
    9.081698408408556e-06,
    2.3000000126671694e-08,
    6.30000016599297e-08,
    7.040000014058023e-07,
    2.499999993688107e-07,
    7.900000014160469e-07,
    2.414212942123413,
    1.4966057538986206,
    1.0,
    0.6681786179542542,
    0.4142135679721832,
    0.1989123672246933,
    5.620000251838064e-07,
    0.0,
    0.0,
    0.0,
    0.0,
    0.0,
    0.0,
};

#ifdef __MWERKS__
float atanf(float x)
{
    float const silver_ratio = 2.4142136573791504f;
    float const silver_ratio_conjugate = 0.4142135679721832f;

    float result;
    const float* lookup_ptr;
    s32 lookup_index = -1;
    bool x_ge_ratio = false;
    s32 sign_bit_x = BITWISE(x) & SIGN_BIT;

    BITWISE(x) &= ~SIGN_BIT;

    if (x >= silver_ratio) {
        x_ge_ratio = true;
        result = 1.0f / x;
    } else if (silver_ratio_conjugate < x) {
        lookup_index = 0;
        switch (BITWISE(x) & BITWISE_INF) {
        case BITWISE_0_5: {
            if (!(SIGNED_BITWISE(x) < BITWISE_THRESHOLD_0)) {
                lookup_index = 1;
            }

            if (!(SIGNED_BITWISE(x) < BITWISE_THRESHOLD_1)) {
                lookup_index += 1;
            }

            break;
        }
        case BITWISE_1_0: {
            lookup_index = 2;
            if (!(SIGNED_BITWISE(x) < BITWISE_THRESHOLD_2)) {
                lookup_index = 3;
            }

            if (!(SIGNED_BITWISE(x) < BITWISE_THRESHOLD_3)) {
                lookup_index += 1;
            }

            break;
        }
        case BITWISE_2_0: {
            lookup_index = 4;
            break;
        }
        }
        {
            float offset_39;
            float offset_33;
            lookup_ptr = &atanf_lookup[lookup_index];
            offset_39 = lookup_ptr[39];
            offset_33 = lookup_ptr[33];

            result = 1.0f / (offset_33 + (x + offset_39));
            result = __fnmsubs(result, lookup_ptr[7], offset_33) +
                     __fnmsubs(result, lookup_ptr[13], offset_39);
        }
    } else {
        result = x;
    }

    {
        float result_squared = result * result;
        lookup_ptr = &atanf_lookup[lookup_index];

        /* retail 0x80022FD0: fmuls result, result_squared; 0x80022FC8,
         * 0x80022FD8, 0x80022FE0, 0x80022FE4, 0x80022FE8: five fmadds
         * (Horner in result_squared); 0x80022FEC: fmadds result^3, poly,
         * result. */
        {
            float result_cubed = result * result_squared;
            float poly = gekko_fmadds(result_squared, atanf_lookup[6], atanf_lookup[5]);
            poly = gekko_fmadds(result_squared, poly, atanf_lookup[4]);
            poly = gekko_fmadds(result_squared, poly, atanf_lookup[3]);
            poly = gekko_fmadds(result_squared, poly, atanf_lookup[2]);
            poly = gekko_fmadds(result_squared, poly, atanf_lookup[1]);
            result = gekko_fmadds(result_cubed, poly, result);
        }

        result += lookup_ptr[27];
        result += lookup_ptr[20];
    }

    if (x_ge_ratio) {
        result -= (float) M_PI_2;
        return sign_bit_x ? result : -result;
    }

    BITWISE(result) |= sign_bit_x;
    return result;
}
#endif
