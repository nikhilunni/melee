/*
 * RETAIL-FAITHFUL COPY of ../trigf.c (itself a verbatim copy of
 * third_party/melee-decomp/src/MSL/trigf.c). Not verbatim: every place the
 * retail disassembly of main.dol shows a fused multiply-add is rewritten
 * with the gekko_fmadds/gekko_fnmsubs/... helpers from
 * tests/ref/gekko_fma.h (included through math.h), each tagged with the
 * instruction address, and int-to-float conversions follow the emitted
 * code rather than C's rules where the two differ. Everything else is
 * untouched; `diff ../trigf.c trigf.c` shows exactly the audited sites.
 *
 * This is what tests/ref_oracle.rs compares the Rust against. The verbatim
 * copy stays alongside so the diff, and the count of inputs on which the
 * two builds disagree, remain reviewable.
 */
#include "math.h"

#define __epsilon 3.45266983e-4f

#define __HI(x) (((s32*) &x)[0])

extern f32 __sincos_on_quadrant[];
extern f32 __sincos_poly[];

const f32 tmp_float[] = { 0.25f, 0.0232393741608f, 1.70555722434e-7f,
                          1.86736494323e-11f };
f32 __four_over_pi_m1[] = { 0.0f, 0.0f, 0.0f, 0.0f };

void __sinit_trigf_c(void)
{
    __four_over_pi_m1[0] = tmp_float[0];
    __four_over_pi_m1[1] = tmp_float[1];
    __four_over_pi_m1[2] = tmp_float[2];
    __four_over_pi_m1[3] = tmp_float[3];
}

SECTION_CTORS void* const __sinit_trigf_c_reference = __sinit_trigf_c;

f32 sinf(f32 x)
{
    int n;
    f32 y;
    f32 ysq;
    f32 z;

    z = (2.0f / (f32) M_PI) * x;
    n = (__HI(x) & 0x80000000) ? (int) (z - 0.5f) : (int) (z + 0.5f);

    /* retail 0x8032646C (sinf) / 0x803262D8 (cosf): fsubs of x and the
     * *double* (n * 2); MWCC never rounds the integer to single first.
     * Then fmadds at 0x80326470..0x8032647C (sinf) / 0x803262DC..0x803262E8
     * (cosf). */
    y = (f32) ((f64) x - (f64) (n * 2));
    y = gekko_fmadds(__four_over_pi_m1[0], x, y);
    y = gekko_fmadds(__four_over_pi_m1[1], x, y);
    y = gekko_fmadds(__four_over_pi_m1[2], x, y);
    y = gekko_fmadds(__four_over_pi_m1[3], x, y);
    n &= 3;

    if (fabsf__Ff(y) < __epsilon) {
        n <<= 1;
        /* retail 0x803264B4 fmuls, 0x803264BC fmadds p9, (q[n+1] * y), q[n] */
        return gekko_fmadds(__sincos_poly[9], __sincos_on_quadrant[n + 1] * y,
                            __sincos_on_quadrant[n]);
    }

    ysq = y * y;
    if (n & 1) {
        n <<= 1;
        /* retail 0x803264EC..0x8032650C (sinf) / 0x80326394..0x803263B4
         * (cosf): four fmadds */
        z = gekko_fmadds(__sincos_poly[0], ysq, __sincos_poly[2]);
        z = gekko_fmadds(ysq, z, __sincos_poly[4]);
        z = gekko_fmadds(ysq, z, __sincos_poly[6]);
        z = gekko_fmadds(ysq, z, __sincos_poly[8]);

        return z * __sincos_on_quadrant[n];
    } else {
        n <<= 1;
        /* retail 0x80326534..0x80326554: four fmadds, then fmuls y */
        z = gekko_fmadds(__sincos_poly[1], ysq, __sincos_poly[3]);
        z = gekko_fmadds(ysq, z, __sincos_poly[5]);
        z = gekko_fmadds(ysq, z, __sincos_poly[7]);
        z = gekko_fmadds(ysq, z, __sincos_poly[9]) * y;
        return z * __sincos_on_quadrant[n + 1];
    }
}

f32 cosf(f32 x)
{
    int n;
    f32 y;
    f32 ysq;
    f32 z;

    z = (2.0f / (f32) M_PI) * x;
    n = (__HI(x) & 0x80000000) ? (int) (z - 0.5f) : (int) (z + 0.5f);

    /* retail 0x8032646C (sinf) / 0x803262D8 (cosf): fsubs of x and the
     * *double* (n * 2); MWCC never rounds the integer to single first.
     * Then fmadds at 0x80326470..0x8032647C (sinf) / 0x803262DC..0x803262E8
     * (cosf). */
    y = (f32) ((f64) x - (f64) (n * 2));
    y = gekko_fmadds(__four_over_pi_m1[0], x, y);
    y = gekko_fmadds(__four_over_pi_m1[1], x, y);
    y = gekko_fmadds(__four_over_pi_m1[2], x, y);
    y = gekko_fmadds(__four_over_pi_m1[3], x, y);
    n &= 3;
    if (fabsf__Ff(y) < __epsilon) {
        n <<= 1;
        /* retail 0x80326318: fnmsubs y, q[n], q[n+1] */
        return gekko_fnmsubs(y, __sincos_on_quadrant[n], __sincos_on_quadrant[n + 1]);
    }

    ysq = y * y;
    if (n & 1) {
        n <<= 1;
        /* retail 0x80326348..0x80326364: three fmadds; 0x80326368 fnmadds
         * absorbs the leading minus; then fmuls y */
        z = gekko_fmadds(__sincos_poly[1], ysq, __sincos_poly[3]);
        z = gekko_fmadds(ysq, z, __sincos_poly[5]);
        z = gekko_fmadds(ysq, z, __sincos_poly[7]);
        z = gekko_fnmadds(ysq, z, __sincos_poly[9]) * y;
        return z * __sincos_on_quadrant[n];
    } else {
        n <<= 1;
        /* retail 0x803264EC..0x8032650C (sinf) / 0x80326394..0x803263B4
         * (cosf): four fmadds */
        z = gekko_fmadds(__sincos_poly[0], ysq, __sincos_poly[2]);
        z = gekko_fmadds(ysq, z, __sincos_poly[4]);
        z = gekko_fmadds(ysq, z, __sincos_poly[6]);
        z = gekko_fmadds(ysq, z, __sincos_poly[8]);
        return z * __sincos_on_quadrant[n + 1];
    }
}

#ifdef MUST_MATCH
#pragma push
#pragma dont_inline on
#endif
f32 sin__Ff(f32 x)
{
    return sinf(x);
}

f32 cos__Ff(f32 x)
{
    return cosf(x);
}
#ifdef MUST_MATCH
#pragma pop
#endif

f32 tanf(f32 x)
{
    return sin__Ff(x) / cos__Ff(x);
}
