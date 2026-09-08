/*
 * Host shim for the decomp's extern/dolphin/include/dolphin/mtx.h.
 *
 * Types are verbatim. The MTX and VEC macros map to the PS-prefixed names
 * exactly as the retail (non-DEBUG) build does; the PS bodies are C
 * re-transcriptions of the paired-single asm, defined in driver.c.
 */
#ifndef HSD_ANIM_REF_MTX_DOLPHIN_MTX_H
#define HSD_ANIM_REF_MTX_DOLPHIN_MTX_H

#include <Runtime/platform.h>

typedef struct {
    f32 x, y, z;
} Vec, Vec3, *VecPtr, Point3d, *Point3dPtr;

typedef struct {
    f32 x, y, z, w;
} Quaternion, Vec4, *QuaternionPtr, Qtrn, *QtrnPtr;

typedef f32 Mtx[3][4];
typedef f32 (*MtxPtr)[4];

#define VECNormalize PSVECNormalize
#define VECSubtract PSVECSubtract
#define VECMag PSVECMag
#define VECDotProduct PSVECDotProduct
#define VECCrossProduct PSVECCrossProduct
#define VECScale PSVECScale
#define MTXCopy PSMTXCopy
#define MTXConcat PSMTXConcat
#define MTXTrans PSMTXTrans
#define MTXScale PSMTXScale
#define MTXQuat PSMTXQuat
#define MTXIdentity PSMTXIdentity

void PSMTXIdentity(Mtx m);
void PSMTXCopy(Mtx src, Mtx dst);
void PSMTXConcat(Mtx mA, Mtx mB, Mtx mAB);
void PSMTXTranspose(Mtx src, Mtx xPose);
void PSMTXScale(Mtx m, f32 xS, f32 yS, f32 zS);
void PSMTXTrans(Mtx m, f32 xT, f32 yT, f32 zT);
void PSMTXQuat(Mtx m, QuaternionPtr q);
void PSMTXRotTrig(Mtx m, char axis, f32 sinA, f32 cosA);
void MTXRotRad(Mtx m, char axis, f32 rad);
u32 PSMTXInverse(Mtx src, Mtx inv);
u32 PSMTXInvXpose(Mtx src, Mtx invX);
void PSMTXMultVec(Mtx m, Vec* src, Vec* dst);
void PSMTXMultVecSR(Mtx m, Vec* src, Vec* dst);

void PSVECAdd(Vec* a, Vec* b, Vec* c);
void PSVECSubtract(Vec* a, Vec* b, Vec* c);
void PSVECScale(Vec* src, Vec* dst, f32 scale);
void PSVECNormalize(Vec* vec1, Vec* dst);
f32 PSVECSquareMag(Vec* vec1);
f32 PSVECMag(Vec* v);
f32 PSVECDotProduct(Vec* vec1, Vec* vec2);
void PSVECCrossProduct(Vec* vec1, Vec* vec2, Vec* dst);

#endif
