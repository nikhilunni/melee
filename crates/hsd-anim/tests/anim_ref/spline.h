/*
 * Shim for sysdolphin/baselib/spline.h. The struct and prototypes are copied
 * from the decomp header; only the include of dolphin/mtx.h is replaced.
 */
#ifndef ANIM_REF_SPLINE_H
#define ANIM_REF_SPLINE_H

#include "platform_shim.h"

typedef struct HSD_Spline {
    /*  +0 */ u8 type;
    /*  +2 */ s16 numcv;
    /*  +4 */ f32 tension;
    /*  +8 */ Vec3* cv;
    /*  +C */ f32 totalLength;
    /* +10 */ f32* segLength;
    /* +14 */ f32 (*segPoly)[5];
} HSD_Spline;

f32 splGetHelmite(f32, f32, f32, f32, f32, f32);
void splGetSplinePoint(Vec3*, HSD_Spline*, f32);
f32 splArcLengthGetParameter(HSD_Spline*, f32);
void splArcLengthPoint(Vec3*, HSD_Spline*, f32);

#endif
