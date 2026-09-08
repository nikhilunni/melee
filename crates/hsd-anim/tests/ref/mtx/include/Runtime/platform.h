/* Host shim for the decomp's Runtime/platform.h: fixed-width typedefs only. */
#ifndef HSD_ANIM_REF_MTX_PLATFORM_H
#define HSD_ANIM_REF_MTX_PLATFORM_H

#include <stdbool.h>
#include <stddef.h>

typedef signed char s8;
typedef unsigned char u8;
typedef signed short s16;
typedef unsigned short u16;
typedef signed int s32;
typedef unsigned int u32;
typedef signed long long s64;
typedef unsigned long long u64;
typedef float f32;
typedef double f64;

#define UNUSED __attribute__((unused))

#endif
