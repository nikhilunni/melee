/*
 * Minimal stand-in for the decomp's Runtime/platform.h and dolphin/mtx.h so
 * the verbatim copies of fobj.c and spline.c compile on the host. Only the
 * names those two files use are provided.
 */
#ifndef ANIM_REF_PLATFORM_SHIM_H
#define ANIM_REF_PLATFORM_SHIM_H

#include <stddef.h>
#include <stdint.h>

typedef uint8_t u8;
typedef int8_t s8;
typedef uint16_t u16;
typedef int16_t s16;
typedef uint32_t u32;
typedef int32_t s32;
typedef float f32;
typedef double f64;
typedef int enum_t;

#ifndef NULL
#define NULL ((void*) 0)
#endif

#define ABS(x) ((x) < 0 ? -(x) : (x))

typedef struct {
    f32 x, y, z;
} Vec3;

#endif
