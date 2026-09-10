/* Numerical body copied from lb/lb_020A.c:244-450. Explicit fused sites:
 * 800214D8..21598 plane projection; 800215C0/CC dot; 800215D0 fnmsubs.
 * Scene fixture supplies the already normalized knee axis. Joint mutation is
 * outside this numerical oracle. Shared audited MSL/Gekko/acos shims below.
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "lbtrigf/retail/lbtrigf.c"
float MSL_TrigF_80400770[1];
float MSL_TrigF_80400774[1];
extern double fabs(double);
typedef struct { float x,y,z; } Vec3;
typedef struct { Vec3 pos0,pos1,pos2,pos3,pos4; float len0,len1; } IKState;
static Vec3 input_axis;
static float result[2];
static Vec3* lbVector_Diff(Vec3* a, Vec3* b, Vec3* out) {
    out->x=a->x-b->x; out->y=a->y-b->y; out->z=a->z-b->z; return out;
}
static float length(Vec3* v) { return sqrtf(v->z*v->z+(v->x*v->x+v->y*v->y)); }
static float lbVector_Angle(Vec3* a, Vec3* b) {
    float product=length(a)*length(b);
    if(product > 1e-10f) {
        /* 8000D748/750 */
        float c=gekko_fmadds(a->z,b->z,gekko_fmadds(a->x,b->x,a->y*b->y))/product;
        if(c>1) c=1; if(c< -1) c=-1;
        return acosf(c);
    }
    return 0;
}
static float calc_acos(float x) { return acosf(x); }
static float sqrtf_store(float x, volatile float* out) { *out=sqrtf(x); return *out; }
void lbBgFlash_80021410(IKState* data)
{
    u8 pad_hi[32];
    Vec3 axis;
    u8 pad_mid[16];
    Vec3 diff_pos0_pos1;
    Vec3 pos1_from_pos0;
    Vec3 temp_delta;
    volatile f32 sin_mag;
    volatile f32 len_ab_mag;
    volatile f32 len_bc_mag;
    volatile f32 len_ac_mag;
    f32 eleven;
    f32 dot;
    f32 sin_val;
    f32 len_ab;
    f32 sum_len;
    f32 len_bc;
    f32 len_ac;
    f32 angle1;
    f32 angle2;
    f32 sum_pow;
    f32 len_pow;
    f32 c2;
    f32 a2;
    f32 b2;
    f32 two_a;
    f32 cos1;
    f32 cos2;
    f32 ten;
    f32 acos1;
    f32 acos2;
    f64 rem;
    f32 dx;
    f32 dz;
    f32 dy;
    f32 last;
    f64 pi;
    Vec3* pDiff;


    axis = input_axis;

    {
        f32 ny = axis.y;
        f32 nz = axis.z;
        f32 nx;
        f32 d;
        f32 x;

        nx = *(f32*) &axis;
        /* 800214D8/EC: fused dot and negated sum. */
        dot = gekko_fnmadds(nz, data->pos1.z, gekko_fmadds(nx, data->pos1.x, ny * data->pos1.y));

        x = data->pos4.x;

        d = -(dot + gekko_fmadds(data->pos4.z, nz, gekko_fmadds(x, nx, data->pos4.y * ny)));
        data->pos4.x = gekko_fmadds(d, nx, x);
        data->pos4.y = gekko_fmadds(d, ny, data->pos4.y);
        data->pos4.z = gekko_fmadds(d, nz, data->pos4.z);

        {
            f32 x = data->pos0.x;
            d = -(dot +
                  gekko_fmadds(data->pos0.z, nz, gekko_fmadds(x, nx, data->pos0.y * ny)));
            data->pos0.x = gekko_fmadds(d, nx, x);
        }
        data->pos0.y = gekko_fmadds(d, ny, data->pos0.y);
        data->pos0.z = gekko_fmadds(d, nz, data->pos0.z);

        {
            f32 x = data->pos1.x;
            d = -(dot +
                  gekko_fmadds(data->pos1.z, nz, gekko_fmadds(x, nx, data->pos1.y * ny)));
            data->pos1.x = gekko_fmadds(d, nx, x);
        }
        data->pos1.y = gekko_fmadds(d, ny, data->pos1.y);
        data->pos1.z = gekko_fmadds(d, nz, data->pos1.z);
    }

    pDiff = lbVector_Diff(&data->pos0, &data->pos1, &diff_pos0_pos1);
    dot = gekko_fmadds(axis.z, pDiff->z, gekko_fmadds(axis.x, pDiff->x, axis.y * pDiff->y));
    sin_val = sqrtf_store(gekko_fnmsubs(dot, dot, 1.0f), &sin_mag);
    data->len0 = data->len0 * sin_val;

    lbVector_Diff(&data->pos4, &data->pos0, &temp_delta);
    lbVector_Diff(&data->pos1, &data->pos0, &pos1_from_pos0);
    angle1 = lbVector_Angle(&temp_delta, &pos1_from_pos0);

    lbVector_Diff(&data->pos2, &data->pos1, &temp_delta);
    angle2 = (f32) ((pi = 3.141592653589793) -
                    lbVector_Angle(&temp_delta, &pos1_from_pos0));

    dx = data->pos0.x - data->pos4.x;
    dz = data->pos0.z;
    dz -= data->pos4.z;
    {
        f32 y = data->pos0.y - data->pos4.y;
        dy = y;
    }
    dx *= dx;
    dy *= dy;
    dz *= dz;
    len_ab = sqrtf_store(dz + (dx + dy), &len_ab_mag);

    dx = data->pos0.x - data->pos1.x;
    dz = data->pos0.z;
    dz -= data->pos1.z;
    dy = data->pos0.y - data->pos1.y;
    dx *= dx;
    dy *= dy;
    dz *= dz;
    len_bc = sqrtf_store(dz + (dx + dy), &len_bc_mag);
    data->len0 = len_bc;

    dx = data->pos1.x - data->pos3.x;
    dz = data->pos1.z;
    dz -= data->pos3.z;
    dy = data->pos1.y - data->pos3.y;
    dx *= dx;
    dy *= dy;
    dz *= dz;
    len_ac = sqrtf_store(dz + (dx + dy), &len_ac_mag);
    data->len1 = len_ac;

    sum_len = (ten = 10.0f) * ((len_bc = data->len0) + (len_ac = data->len1));
    sum_len /= (eleven = 11.0f);
    sum_pow = sum_len * (sum_len * sum_len);
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    sum_pow = sum_len * sum_pow;
    len_pow = len_ab * (len_ab * (len_ab * len_ab));
    len_pow = len_ab * len_pow;
    len_pow = len_ab * len_pow;
    len_pow = len_ab * len_pow;
    len_pow = len_ab * len_pow;
    last = len_ab * (len_pow = len_ab * len_pow);
    if (len_ab > sum_len) {
        len_ab = ((eleven * sum_len) / ten) + (-sum_pow / (ten * last));
    }

    a2 = len_bc * len_bc;
    b2 = len_ab * len_ab;
    two_a = 2.0f * len_bc;
    c2 = len_ac * len_ac;

    cos1 = ((a2 + b2) - c2) / (two_a * len_ab);
    cos2 = ((a2 + c2) - b2) / (two_a * len_ac);

    if (cos1 > 1.0f) {
        cos1 = 1.0f;
    } else if (cos1 < -1.0f) {
        cos1 = -1.0f;
    }

    if (cos2 > 1.0f) {
        cos2 = 1.0f;
    } else if (cos2 < -1.0f) {
        cos2 = -1.0f;
    }

    acos1 = calc_acos(cos1);
    acos2 = calc_acos(cos2);
    rem = 3.141592653589793 - (f64) acos2;
    if (rem < 0.1745329201221466) {
        f32 ratio = (f32) (fabs(rem) / 0.1745329201221466);
        acos2 = (f32) (2.9670597334676465 +
                       (f64) (f32) ((f64) ratio *
                                    ((f64) acos2 - 2.9670597334676465)));
    }

    acos1 -= angle1;
    acos2 -= angle2;
    result[0] = acos1;
    result[1] = acos2;
}
int main(int argc, char** argv) {
    if(argc != 21) return 2;
    float input[20];
    for(int i=0;i<20;i++) { unsigned u; if(sscanf(argv[i+1], "%x", &u)!=1) return 3; memcpy(&input[i],&u,4); }
    IKState data; memcpy(&data,input,17*4); memcpy(&input_axis,input+17,3*4);
    lbBgFlash_80021410(&data);
    for(int i=0;i<2;i++) { unsigned u; memcpy(&u,&result[i],4); printf("%08x\n",u); }
    return 0;
}
