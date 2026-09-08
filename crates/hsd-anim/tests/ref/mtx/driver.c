/*
 * Reference-oracle driver for hsd-anim's mtx/quat port. Built by
 * tests/mtx_oracle.rs together with the decomp's own
 *   src/sysdolphin/baselib/mtx.c
 *   src/sysdolphin/baselib/quatlib.c
 * and gekko-math's verbatim copy of MSL trigf.c (for sinf/cosf), all with
 * `-O0 -ffp-contract=off`.
 *
 * What is native decomp C: every HSD_Mtx*, MatToQuat, EulerToQuat and
 * HSD_QuatLib_* body, plus sinf/cosf.
 *
 * What is re-transcribed here, because Melee links paired-single asm for it
 * and asm cannot be compiled on a host:
 *   - PSMTX* / PSVEC*: one C statement per asm instruction, fused steps via
 *     fmaf. This is an independent transcription of the same asm the Rust
 *     was written from, so a mismatch means one of the two misread it; a
 *     match does not prove either read is right.
 *   - sqrtf: MSL's math_ppc.h algorithm over gekko-math's *placeholder*
 *     frsqrte (IEEE 1/sqrt), spelled the same way gekko_math::msl::sqrtf is.
 *   - fres placeholder: 1.0f / x, as in gekko_math::estimate::fres.
 *   - atan2f / asinf / acosf: deterministic stand-ins (these are Melee's own
 *     lbtrigf.c, out of scope here). The Rust test passes the same stand-ins
 *     through the InverseTrig trait, so the surrounding arithmetic and
 *     control flow are still compared bit for bit.
 *
 * Usage: driver <op> <in-file> <out-file>
 * Records are host-endian raw f32 words; each op has a fixed input and
 * output word count listed in the table at the bottom and mirrored in
 * mtx_oracle.rs.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <math.h> /* the shim in include/ */
#include <dolphin/mtx.h>
#include <sysdolphin/baselib/objalloc.h>

/* Real libm entry points we need, declared by hand so the shim <math.h>
 * stays in charge of the single-precision names. */
double sqrt(double);
float fmaf(float, float, float);

/* MSL trigf.c fills its range-reduction tables from a static constructor;
 * on the host it has to be called explicitly before any sinf/cosf. */
void __sinit_trigf_c(void);

/* Decomp headers (baselib) for the HSD prototypes. */
void HSD_MtxInverse(Mtx src, Mtx dest);
void HSD_MtxInverseConcat(Mtx inv, Mtx src, Mtx dest);
void HSD_MtxInverseTranspose(Mtx src, Mtx dest);
void HSD_MtxGetRotation(Mtx m, Vec3* vec);
void HSD_MtxGetTranslate(Mtx mat, Vec3* vec);
void HSD_MtxGetScale(Mtx arg0, Vec3* arg1);
void HSD_MkRotationMtx(Mtx arg0, Vec3* arg1);
void HSD_MtxQuat(Mtx arg0, Quaternion* arg1);
void HSD_MtxSRT(Mtx m, Vec3* vec1, Vec3* vec2, Vec3* vec3, Vec3* vec4);
void HSD_MtxSRTQuat(Mtx arg0, Vec3* arg1, Quaternion* arg2, Vec3* arg3,
                    Vec3* arg4);
void HSD_MtxScaledAdd(Mtx arg0, Mtx arg1, Mtx arg2, f32 arg3);
s32 MatToQuat(Mtx, Quaternion*);
s32 HSD_QuatLib_8037EB28(Mtx, Vec3*);
s32 HSD_QuatLib_8037EC4C(Quaternion*, Quaternion*, Quaternion*);
s32 HSD_QuatLib_8037ECE0(Vec3*, Quaternion*, f32);
s32 EulerToQuat(Vec3*, Quaternion*);
s32 HSD_QuatLib_8037EF28(Quaternion*, Quaternion*, Quaternion*, f32);

/* ------------------------------------------------------------------------ */
/* Allocator stubs referenced by mtx.c (never called by the oracle).        */

void* HSD_ObjAlloc(HSD_ObjAllocData* data)
{
    (void) data;
    abort();
}
void HSD_ObjFree(HSD_ObjAllocData* data, void* obj)
{
    (void) data;
    (void) obj;
}
void HSD_ObjAllocInit(HSD_ObjAllocData* data, u32 size, u32 align)
{
    (void) data;
    (void) size;
    (void) align;
}

/* ------------------------------------------------------------------------ */
/* Gekko primitives, mirroring gekko-math.                                  */

static float fmadds(float a, float c, float b)
{
    return fmaf(a, c, b);
}
static float fmsubs(float a, float c, float b)
{
    return fmaf(a, c, -b);
}
static float fnmadds(float a, float c, float b)
{
    return -fmaf(a, c, b);
}
static float fnmsubs(float a, float c, float b)
{
    return -fmaf(a, c, -b);
}
/* ESTIMATE PLACEHOLDER: gekko_math::estimate::frsqrte returns IEEE 1/sqrt. */
static double frsqrte(double x)
{
    return 1.0 / sqrt(x);
}
/* ESTIMATE PLACEHOLDER: gekko_math::estimate::fres returns IEEE 1/x. */
static float fres(float x)
{
    return 1.0f / x;
}

/* gekko_math::msl::sqrtf, spelled identically. */
float sqrtf(float x)
{
    if (x > 0.0f) {
        double xd = (double) x;
        double guess = frsqrte(xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        guess = 0.5 * guess * (3.0 - guess * guess * xd);
        return (float) (xd * guess);
    }
    return x;
}

/* Stand-ins for lbtrigf.c; mirrored by StubTrig in mtx_oracle.rs. */
float atan2f(float y, float x)
{
    return y * 0.5f + x * 0.25f;
}
float asinf(float x)
{
    return x * 0.5f;
}
float acosf(float x)
{
    return 1.0f - x * 0.5f;
}

/* ------------------------------------------------------------------------ */
/* PSVEC* : vec.c asm, one statement per instruction.                       */

void PSVECAdd(Vec* a, Vec* b, Vec* c)
{
    Vec r;
    r.x = a->x + b->x;
    r.y = a->y + b->y;
    r.z = a->z + b->z;
    *c = r;
}

void PSVECSubtract(Vec* a, Vec* b, Vec* c)
{
    Vec r;
    r.x = a->x - b->x;
    r.y = a->y - b->y;
    r.z = a->z - b->z;
    *c = r;
}

void PSVECScale(Vec* src, Vec* dst, f32 mult)
{
    Vec r;
    r.x = src->x * mult;
    r.y = src->y * mult;
    r.z = src->z * mult;
    *dst = r;
}

f32 PSVECSquareMag(Vec* v)
{
    float xx = v->x * v->x;
    float yy = v->y * v->y;
    return fmadds(v->z, v->z, xx) + yy;
}

f32 PSVECMag(Vec* v)
{
    float xx = v->x * v->x;
    float yy = v->y * v->y;
    float sqsum = fmadds(v->z, v->z, xx) + yy;
    double est = frsqrte((double) sqsum);
    float f2 = (float) (est * est);
    float f0 = (float) (est * 0.5);
    f2 = fnmsubs(f2, sqsum, 3.0f);
    f0 = f2 * f0;
    f0 = (f0 >= 0.0f) ? f0 : sqsum; /* fsel */
    return sqsum * f0;
}

void PSVECNormalize(Vec* src, Vec* dst)
{
    float xx = src->x * src->x;
    float yy = src->y * src->y;
    float sqsum = fmadds(src->z, src->z, xx) + yy;
    double rsqrt = frsqrte((double) sqsum);
    float nwork0 = (float) (rsqrt * rsqrt);
    float nwork1 = (float) (rsqrt * 0.5);
    float r;
    Vec out;
    nwork0 = fnmsubs(nwork0, sqsum, 3.0f);
    r = nwork0 * nwork1;
    out.x = src->x * r;
    out.y = src->y * r;
    out.z = src->z * r;
    *dst = out;
}

f32 PSVECDotProduct(Vec* a, Vec* b)
{
    float yy = a->y * b->y;
    float zz = a->z * b->z;
    return fmadds(a->x, b->x, yy) + zz;
}

void PSVECCrossProduct(Vec* a, Vec* b, Vec* dst)
{
    float f4_0 = b->x * a->z;
    float f4_1 = b->y * a->z;
    float f7_1 = b->y * a->x;
    float f5_0 = fmsubs(a->x, b->z, f4_0);
    float f5_1 = fmsubs(a->y, b->z, f4_1);
    float f8_1 = fmsubs(a->y, b->x, f7_1);
    Vec out;
    out.x = f5_1;
    out.y = -f5_0;
    out.z = -f8_1;
    *dst = out;
}

/* ------------------------------------------------------------------------ */
/* PSMTX* : mtx.c / mtxvec.c asm.                                           */

void PSMTXIdentity(Mtx m)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = 1.0f; m[1][2] = 0.0f; m[1][3] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 1.0f; m[2][3] = 0.0f;
}

void PSMTXCopy(Mtx src, Mtx dst)
{
    memmove(dst, src, sizeof(Mtx));
}

void PSMTXConcat(Mtx mA, Mtx mB, Mtx mAB)
{
    static const float Unit01[2] = { 0.0f, 1.0f };
    Mtx a, b, out;
    int i, j;
    memcpy(a, mA, sizeof(Mtx));
    memcpy(b, mB, sizeof(Mtx));
    for (i = 0; i < 3; i++) {
        for (j = 0; j < 4; j++) {
            float v = b[0][j] * a[i][0];
            v = fmadds(b[1][j], a[i][1], v);
            v = fmadds(b[2][j], a[i][2], v);
            out[i][j] = v;
        }
        out[i][2] = fmadds(Unit01[0], a[i][3], out[i][2]);
        out[i][3] = fmadds(Unit01[1], a[i][3], out[i][3]);
    }
    memcpy(mAB, out, sizeof(Mtx));
}

void PSMTXTranspose(Mtx src, Mtx xPose)
{
    Mtx s, out;
    memcpy(s, src, sizeof(Mtx));
    out[0][0] = s[0][0]; out[0][1] = s[1][0]; out[0][2] = s[2][0]; out[0][3] = 0.0f;
    out[1][0] = s[0][1]; out[1][1] = s[1][1]; out[1][2] = s[2][1]; out[1][3] = 0.0f;
    out[2][0] = s[0][2]; out[2][1] = s[1][2]; out[2][2] = s[2][2]; out[2][3] = 0.0f;
    memcpy(xPose, out, sizeof(Mtx));
}

/* Shared prologue of PSMTXInverse / PSMTXInvXpose. */
static float ps_cofactors(Mtx m, float f13[2], float f12[2], float f11[2],
                          float* f10, float* f9, float* f8)
{
    float m00 = m[0][0], m01 = m[0][1], m02 = m[0][2];
    float m10 = m[1][0], m11 = m[1][1], m12 = m[1][2];
    float m20 = m[2][0], m21 = m[2][1], m22 = m[2][2];
    float det;

    f11[0] = m11 * m02;
    f11[1] = m12 * m00;
    f13[0] = m21 * m12;
    f13[1] = m22 * m10;
    f11[0] = fmsubs(m01, m12, f11[0]);
    f11[1] = fmsubs(m02, m10, f11[1]);
    f12[0] = m01 * m22;
    f12[1] = m02 * m20;
    f13[0] = fmsubs(m11, m22, f13[0]);
    f13[1] = fmsubs(m12, m20, f13[1]);
    *f10 = m11 * m20;
    f12[0] = fmsubs(m21, m02, f12[0]);
    f12[1] = fmsubs(m22, m00, f12[1]);
    *f9 = m00 * m21;
    *f8 = m01 * m10;
    *f10 = fmsubs(m10, m21, *f10);
    *f9 = fmsubs(m01, m20, *f9);
    *f8 = fmsubs(m00, m11, *f8);

    det = m00 * f13[0];
    det = fmadds(m10, f12[0], det);
    det = fmadds(m20, f11[0], det);
    return det;
}

u32 PSMTXInverse(Mtx src, Mtx inv)
{
    float f13[2], f12[2], f11[2], f10, f9, f8;
    float det, r, f6, f5, rdet;
    float s03, s13, s23;
    float i00, i01, i02, i10, i11, i12, i20, i21, i22, i03, i13, i23;
    Mtx s;
    memcpy(s, src, sizeof(Mtx));

    det = ps_cofactors(s, f13, f12, f11, &f10, &f9, &f8);
    if (det == 0.0f) {
        return 0;
    }
    r = fres(det);
    f6 = r + r;
    f5 = r * r;
    rdet = fnmsubs(det, f5, f6);

    s03 = s[0][3];
    s13 = s[1][3];
    s23 = s[2][3];

    i00 = f13[0] * rdet;
    i10 = f13[1] * rdet;
    i01 = f12[0] * rdet;
    i11 = f12[1] * rdet;
    i02 = f11[0] * rdet;
    i12 = f11[1] * rdet;
    i20 = f10 * rdet;
    i21 = f9 * rdet;
    i22 = f8 * rdet;

    i03 = fnmadds(i02, s23, fmadds(i01, s13, i00 * s03));
    i13 = fnmadds(i12, s23, fmadds(i11, s13, i10 * s03));
    i23 = fnmadds(i22, s23, fmadds(i21, s13, i20 * s03));

    inv[0][0] = i00; inv[0][1] = i01; inv[0][2] = i02; inv[0][3] = i03;
    inv[1][0] = i10; inv[1][1] = i11; inv[1][2] = i12; inv[1][3] = i13;
    inv[2][0] = i20; inv[2][1] = i21; inv[2][2] = i22; inv[2][3] = i23;
    return 1;
}

u32 PSMTXInvXpose(Mtx src, Mtx invX)
{
    float f13[2], f12[2], f11[2], f10, f9, f8;
    float det, r, f6, f5, rdet;
    Mtx s;
    memcpy(s, src, sizeof(Mtx));

    det = ps_cofactors(s, f13, f12, f11, &f10, &f9, &f8);
    if (det == 0.0f) {
        return 0;
    }
    r = fres(det);
    f6 = r + r;
    f5 = r * r;
    r = fnmsubs(det, f5, f6);
    f6 = r + r;
    f5 = r * r;
    rdet = fnmsubs(det, f5, f6);

    invX[0][0] = f13[0] * rdet; invX[0][1] = f13[1] * rdet; invX[0][2] = f10 * rdet; invX[0][3] = 0.0f;
    invX[1][0] = f12[0] * rdet; invX[1][1] = f12[1] * rdet; invX[1][2] = f9 * rdet;  invX[1][3] = 0.0f;
    invX[2][0] = f11[0] * rdet; invX[2][1] = f11[1] * rdet; invX[2][2] = f8 * rdet;  invX[2][3] = 0.0f;
    return 1;
}

void PSMTXRotTrig(Mtx m, char axis, f32 sinA, f32 cosA)
{
    float nsinA = -sinA;
    switch ((unsigned char) axis | 0x20) {
    case 'x':
        m[0][0] = 1.0f; m[0][1] = 0.0f;  m[0][2] = 0.0f;  m[0][3] = 0.0f;
        m[1][0] = 0.0f; m[1][1] = cosA;  m[1][2] = nsinA; m[1][3] = 0.0f;
        m[2][0] = 0.0f; m[2][1] = sinA;  m[2][2] = cosA;  m[2][3] = 0.0f;
        break;
    case 'y':
        m[0][0] = cosA;  m[0][1] = 0.0f; m[0][2] = sinA; m[0][3] = 0.0f;
        m[1][0] = 0.0f;  m[1][1] = 1.0f; m[1][2] = 0.0f; m[1][3] = 0.0f;
        m[2][0] = nsinA; m[2][1] = 0.0f; m[2][2] = cosA; m[2][3] = 0.0f;
        break;
    case 'z':
        m[0][0] = cosA; m[0][1] = nsinA; m[0][2] = 0.0f; m[0][3] = 0.0f;
        m[1][0] = sinA; m[1][1] = cosA;  m[1][2] = 0.0f; m[1][3] = 0.0f;
        m[2][0] = 0.0f; m[2][1] = 0.0f;  m[2][2] = 1.0f; m[2][3] = 0.0f;
        break;
    default:
        break;
    }
}

void MTXRotRad(Mtx m, char axis, f32 rad)
{
    f32 sinA = sinf(rad);
    f32 cosA = cosf(rad);
    PSMTXRotTrig(m, axis, sinA, cosA);
}

void PSMTXTrans(Mtx m, f32 xT, f32 yT, f32 zT)
{
    m[0][0] = 1.0f; m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = xT;
    m[1][0] = 0.0f; m[1][1] = 1.0f; m[1][2] = 0.0f; m[1][3] = yT;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = 1.0f; m[2][3] = zT;
}

void PSMTXScale(Mtx m, f32 xS, f32 yS, f32 zS)
{
    m[0][0] = xS;   m[0][1] = 0.0f; m[0][2] = 0.0f; m[0][3] = 0.0f;
    m[1][0] = 0.0f; m[1][1] = yS;   m[1][2] = 0.0f; m[1][3] = 0.0f;
    m[2][0] = 0.0f; m[2][1] = 0.0f; m[2][2] = zS;   m[2][3] = 0.0f;
}

void PSMTXQuat(Mtx m, QuaternionPtr q)
{
    float x = q->x, y = q->y, z = q->z, w = q->w;
    float c_one = 1.0f;
    float c_two = c_one + c_one;
    float c_zero = c_one - c_one;
    float xx, yy, zz, tmp4_0, tmp4_1, scale, yw, xw, tmp9, zw, tmp2_0;
    float tmp8_0, tmp6_0, tmp5_0, tmp5_1, tmp7_0, tmp7_1;

    xx = x * x;
    yy = y * y;
    tmp4_0 = fmadds(z, z, xx);
    tmp4_1 = fmadds(w, w, yy);
    zz = z * z;
    scale = tmp4_0 + tmp4_1;
    yw = y * w;
    xw = x * w;
    tmp9 = fres(scale);
    tmp4_1 = zz + yy;
    scale = fnmsubs(scale, tmp9, c_two);
    zw = z * w;
    scale = tmp9 * scale;
    tmp2_0 = xx + yy;
    scale = scale * c_two;
    tmp8_0 = fmadds(x, y, zw);
    tmp6_0 = fmsubs(x, y, zw);
    tmp2_0 = fnmsubs(tmp2_0, scale, c_one);
    tmp4_0 = fnmsubs(tmp4_0, scale, c_one);
    tmp4_1 = fnmsubs(tmp4_1, scale, c_one);
    tmp8_0 = tmp8_0 * scale;
    tmp6_0 = tmp6_0 * scale;
    tmp5_0 = fmadds(x, z, yw);
    tmp5_1 = fmadds(y, z, xw);
    tmp7_0 = fnmsubs(yw, c_two, tmp5_0);
    tmp7_1 = fnmsubs(xw, c_two, tmp5_1);
    tmp5_0 = tmp5_0 * scale;
    tmp5_1 = tmp5_1 * scale;
    tmp7_0 = tmp7_0 * scale;
    tmp7_1 = tmp7_1 * scale;

    m[0][0] = tmp4_1; m[0][1] = tmp6_0; m[0][2] = tmp5_0; m[0][3] = c_zero;
    m[1][0] = tmp8_0; m[1][1] = tmp4_0; m[1][2] = tmp7_1; m[1][3] = c_zero;
    m[2][0] = tmp7_0; m[2][1] = tmp5_1; m[2][2] = tmp2_0; m[2][3] = c_zero;
}

static float multvec_row(const float* r, float x, float y, float z)
{
    float f4_0 = r[0] * x;
    float f4_1 = r[1] * y;
    return fmadds(r[2], z, f4_0) + fmadds(r[3], 1.0f, f4_1);
}

void PSMTXMultVec(Mtx m, Vec* src, Vec* dst)
{
    float x = src->x, y = src->y, z = src->z;
    Vec out;
    out.x = multvec_row(m[0], x, y, z);
    out.y = multvec_row(m[1], x, y, z);
    out.z = multvec_row(m[2], x, y, z);
    *dst = out;
}

static float multvecsr_row(const float* r, float x, float y, float z)
{
    float f8 = r[0] * x + r[1] * y;
    return fmadds(r[2], z, f8);
}

void PSMTXMultVecSR(Mtx m, Vec* src, Vec* dst)
{
    float x = src->x, y = src->y, z = src->z;
    Vec out;
    out.x = multvecsr_row(m[0], x, y, z);
    out.y = multvecsr_row(m[1], x, y, z);
    out.z = multvecsr_row(m[2], x, y, z);
    *dst = out;
}

/* ------------------------------------------------------------------------ */
/* Ops.                                                                      */

#define SENTINEL 12345.678f

static void get_mtx(const float* in, Mtx m)
{
    memcpy(m, in, 12 * sizeof(float));
}
static void put_mtx(float* out, Mtx m)
{
    memcpy(out, m, 12 * sizeof(float));
}
static Vec get_vec(const float* in)
{
    Vec v;
    v.x = in[0];
    v.y = in[1];
    v.z = in[2];
    return v;
}
static void put_vec(float* out, const Vec* v)
{
    out[0] = v->x;
    out[1] = v->y;
    out[2] = v->z;
}
static Quaternion get_quat(const float* in)
{
    Quaternion q;
    q.x = in[0];
    q.y = in[1];
    q.z = in[2];
    q.w = in[3];
    return q;
}
static void put_quat(float* out, const Quaternion* q)
{
    out[0] = q->x;
    out[1] = q->y;
    out[2] = q->z;
    out[3] = q->w;
}
static void fill_mtx(Mtx m)
{
    int i, j;
    for (i = 0; i < 3; i++)
        for (j = 0; j < 4; j++)
            m[i][j] = SENTINEL;
}

/* SDK */
static void op_vec_sqmag(const float* in, float* out)
{
    Vec v = get_vec(in);
    out[0] = PSVECSquareMag(&v);
}
static void op_vec_mag(const float* in, float* out)
{
    Vec v = get_vec(in);
    out[0] = PSVECMag(&v);
}
static void op_vec_normalize(const float* in, float* out)
{
    Vec v = get_vec(in), d;
    PSVECNormalize(&v, &d);
    put_vec(out, &d);
}
static void op_vec_dot(const float* in, float* out)
{
    Vec a = get_vec(in), b = get_vec(in + 3);
    out[0] = PSVECDotProduct(&a, &b);
}
static void op_vec_cross(const float* in, float* out)
{
    Vec a = get_vec(in), b = get_vec(in + 3), d;
    PSVECCrossProduct(&a, &b, &d);
    put_vec(out, &d);
}
static void op_vec_addsubscale(const float* in, float* out)
{
    Vec a = get_vec(in), b = get_vec(in + 3), d;
    PSVECAdd(&a, &b, &d);
    put_vec(out, &d);
    PSVECSubtract(&a, &b, &d);
    put_vec(out + 3, &d);
    PSVECScale(&a, &d, in[6]);
    put_vec(out + 6, &d);
}
static void op_mtx_concat(const float* in, float* out)
{
    Mtx a, b, ab;
    get_mtx(in, a);
    get_mtx(in + 12, b);
    PSMTXConcat(a, b, ab);
    put_mtx(out, ab);
}
static void op_mtx_transpose(const float* in, float* out)
{
    Mtx a, t;
    get_mtx(in, a);
    PSMTXTranspose(a, t);
    put_mtx(out, t);
}
static void op_mtx_inverse(const float* in, float* out)
{
    Mtx a, inv;
    get_mtx(in, a);
    fill_mtx(inv);
    out[0] = (float) PSMTXInverse(a, inv);
    put_mtx(out + 1, inv);
}
static void op_mtx_invxpose(const float* in, float* out)
{
    Mtx a, inv;
    get_mtx(in, a);
    fill_mtx(inv);
    out[0] = (float) PSMTXInvXpose(a, inv);
    put_mtx(out + 1, inv);
}
static void op_mtx_quat(const float* in, float* out)
{
    Quaternion q = get_quat(in);
    Mtx m;
    PSMTXQuat(m, &q);
    put_mtx(out, m);
}
static void op_mtx_multvec(const float* in, float* out)
{
    Mtx m;
    Vec v = get_vec(in + 12), d;
    get_mtx(in, m);
    PSMTXMultVec(m, &v, &d);
    put_vec(out, &d);
}
static void op_mtx_multvecsr(const float* in, float* out)
{
    Mtx m;
    Vec v = get_vec(in + 12), d;
    get_mtx(in, m);
    PSMTXMultVecSR(m, &v, &d);
    put_vec(out, &d);
}
static void op_mtx_rotrad(const float* in, float* out)
{
    Mtx m;
    fill_mtx(m);
    MTXRotRad(m, (char) (int) in[0], in[1]);
    put_mtx(out, m);
}
static void op_mtx_scale_trans(const float* in, float* out)
{
    Mtx m;
    PSMTXScale(m, in[0], in[1], in[2]);
    put_mtx(out, m);
    PSMTXTrans(m, in[0], in[1], in[2]);
    put_mtx(out + 12, m);
}

/* HSD */
static void op_hsd_inverse(const float* in, float* out)
{
    Mtx a, d;
    get_mtx(in, a);
    HSD_MtxInverse(a, d);
    put_mtx(out, d);
}
static void op_hsd_inverse_concat(const float* in, float* out)
{
    Mtx a, b, d;
    get_mtx(in, a);
    get_mtx(in + 12, b);
    HSD_MtxInverseConcat(a, b, d);
    put_mtx(out, d);
}
static void op_hsd_inverse_transpose(const float* in, float* out)
{
    Mtx a, d;
    get_mtx(in, a);
    HSD_MtxInverseTranspose(a, d);
    put_mtx(out, d);
}
static void op_hsd_get_rotation(const float* in, float* out)
{
    Mtx a;
    Vec3 v = { SENTINEL, SENTINEL, SENTINEL };
    get_mtx(in, a);
    HSD_MtxGetRotation(a, &v);
    put_vec(out, &v);
}
static void op_hsd_get_translate(const float* in, float* out)
{
    Mtx a;
    Vec3 v;
    get_mtx(in, a);
    HSD_MtxGetTranslate(a, &v);
    put_vec(out, &v);
}
static void op_hsd_get_scale(const float* in, float* out)
{
    Mtx a;
    Vec3 v;
    get_mtx(in, a);
    HSD_MtxGetScale(a, &v);
    put_vec(out, &v);
}
static void op_hsd_mk_rotation(const float* in, float* out)
{
    Mtx m;
    Vec3 r = get_vec(in);
    HSD_MkRotationMtx(m, &r);
    put_mtx(out, m);
}
static void op_hsd_mtx_quat(const float* in, float* out)
{
    Quaternion q = get_quat(in);
    Mtx m;
    HSD_MtxQuat(m, &q);
    put_mtx(out, m);
}
/* in: scale[3] rot[3] trans[3] flag vec4[3] */
static void op_hsd_srt(const float* in, float* out)
{
    Mtx m;
    Vec3 s = get_vec(in), r = get_vec(in + 3), t = get_vec(in + 6),
         v4 = get_vec(in + 10);
    HSD_MtxSRT(m, &s, &r, &t, in[9] != 0.0f ? &v4 : NULL);
    put_mtx(out, m);
}
/* in: scale[3] quat[4] trans[3] flag vec4[3] */
static void op_hsd_srt_quat(const float* in, float* out)
{
    Mtx m;
    Vec3 s = get_vec(in), t = get_vec(in + 7), v4 = get_vec(in + 11);
    Quaternion q = get_quat(in + 3);
    HSD_MtxSRTQuat(m, &s, &q, &t, in[10] != 0.0f ? &v4 : NULL);
    put_mtx(out, m);
}
static void op_hsd_scaled_add(const float* in, float* out)
{
    Mtx a, b, d;
    get_mtx(in, a);
    get_mtx(in + 12, b);
    HSD_MtxScaledAdd(a, b, d, in[24]);
    put_mtx(out, d);
}

/* quatlib */
static void op_mat_to_quat(const float* in, float* out)
{
    Mtx m;
    Quaternion q;
    get_mtx(in, m);
    MatToQuat(m, &q);
    put_quat(out, &q);
}
static void op_mtx_to_euler(const float* in, float* out)
{
    Mtx m;
    Vec3 e;
    get_mtx(in, m);
    HSD_QuatLib_8037EB28(m, &e);
    put_vec(out, &e);
}
static void op_quat_mul(const float* in, float* out)
{
    Quaternion p = get_quat(in), q = get_quat(in + 4), o;
    HSD_QuatLib_8037EC4C(&p, &q, &o);
    put_quat(out, &o);
}
static void op_quat_axis_angle(const float* in, float* out)
{
    Vec3 axis = get_vec(in);
    Quaternion q = { SENTINEL, SENTINEL, SENTINEL, SENTINEL };
    out[0] = (float) (HSD_QuatLib_8037ECE0(&axis, &q, in[3]) == 0);
    put_quat(out + 1, &q);
}
static void op_euler_to_quat(const float* in, float* out)
{
    Vec3 e = get_vec(in);
    Quaternion q;
    EulerToQuat(&e, &q);
    put_quat(out, &q);
}
static void op_quat_slerp(const float* in, float* out)
{
    Quaternion p = get_quat(in), q = get_quat(in + 4), o;
    HSD_QuatLib_8037EF28(&p, &q, &o, in[8]);
    put_quat(out, &o);
}

typedef void (*op_fn)(const float*, float*);
typedef struct {
    const char* name;
    size_t nin;
    size_t nout;
    op_fn fn;
} OpDesc;

/* Mirrored by OPS in mtx_oracle.rs. */
static const OpDesc OPS[] = {
    { "vec_sqmag", 3, 1, op_vec_sqmag },
    { "vec_mag", 3, 1, op_vec_mag },
    { "vec_normalize", 3, 3, op_vec_normalize },
    { "vec_dot", 6, 1, op_vec_dot },
    { "vec_cross", 6, 3, op_vec_cross },
    { "vec_addsubscale", 7, 9, op_vec_addsubscale },
    { "mtx_concat", 24, 12, op_mtx_concat },
    { "mtx_transpose", 12, 12, op_mtx_transpose },
    { "mtx_inverse", 12, 13, op_mtx_inverse },
    { "mtx_invxpose", 12, 13, op_mtx_invxpose },
    { "mtx_quat", 4, 12, op_mtx_quat },
    { "mtx_multvec", 15, 3, op_mtx_multvec },
    { "mtx_multvecsr", 15, 3, op_mtx_multvecsr },
    { "mtx_rotrad", 2, 12, op_mtx_rotrad },
    { "mtx_scale_trans", 3, 24, op_mtx_scale_trans },
    { "hsd_inverse", 12, 12, op_hsd_inverse },
    { "hsd_inverse_concat", 24, 12, op_hsd_inverse_concat },
    { "hsd_inverse_transpose", 12, 12, op_hsd_inverse_transpose },
    { "hsd_get_rotation", 12, 3, op_hsd_get_rotation },
    { "hsd_get_translate", 12, 3, op_hsd_get_translate },
    { "hsd_get_scale", 12, 3, op_hsd_get_scale },
    { "hsd_mk_rotation", 3, 12, op_hsd_mk_rotation },
    { "hsd_mtx_quat", 4, 12, op_hsd_mtx_quat },
    { "hsd_srt", 13, 12, op_hsd_srt },
    { "hsd_srt_quat", 14, 12, op_hsd_srt_quat },
    { "hsd_scaled_add", 25, 12, op_hsd_scaled_add },
    { "mat_to_quat", 12, 4, op_mat_to_quat },
    { "mtx_to_euler", 12, 3, op_mtx_to_euler },
    { "quat_mul", 8, 4, op_quat_mul },
    { "quat_axis_angle", 4, 5, op_quat_axis_angle },
    { "euler_to_quat", 3, 4, op_euler_to_quat },
    { "quat_slerp", 9, 4, op_quat_slerp },
};

static void* slurp(const char* path, size_t* len)
{
    FILE* f = fopen(path, "rb");
    long n;
    void* buf;
    if (!f) {
        perror(path);
        exit(2);
    }
    fseek(f, 0, SEEK_END);
    n = ftell(f);
    fseek(f, 0, SEEK_SET);
    buf = malloc(n ? (size_t) n : 1);
    if (fread(buf, 1, (size_t) n, f) != (size_t) n) {
        perror("fread");
        exit(2);
    }
    fclose(f);
    *len = (size_t) n;
    return buf;
}

int main(int argc, char** argv)
{
    const OpDesc* op = NULL;
    size_t i, len, count;
    float* in;
    float* out;
    FILE* fo;

    if (argc != 4) {
        fprintf(stderr, "usage: %s <op> <in> <out>\n", argv[0]);
        return 2;
    }
    __sinit_trigf_c();
    for (i = 0; i < sizeof(OPS) / sizeof(OPS[0]); i++) {
        if (strcmp(OPS[i].name, argv[1]) == 0) {
            op = &OPS[i];
        }
    }
    if (!op) {
        fprintf(stderr, "unknown op %s\n", argv[1]);
        return 2;
    }
    in = slurp(argv[2], &len);
    if (len % (op->nin * sizeof(float)) != 0) {
        fprintf(stderr, "%s: input length %zu not a multiple of %zu\n",
                op->name, len, op->nin * sizeof(float));
        return 2;
    }
    count = len / (op->nin * sizeof(float));
    out = malloc(count * op->nout * sizeof(float) + 1);
    for (i = 0; i < count; i++) {
        size_t k;
        float* o = out + i * op->nout;
        for (k = 0; k < op->nout; k++) {
            o[k] = SENTINEL;
        }
        op->fn(in + i * op->nin, o);
    }
    fo = fopen(argv[3], "wb");
    if (!fo) {
        perror(argv[3]);
        return 2;
    }
    fwrite(out, sizeof(float), count * op->nout, fo);
    fclose(fo);
    free(in);
    free(out);
    return 0;
}
