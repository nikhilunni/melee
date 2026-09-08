/*
 * Host shim for the decomp's Runtime/platform.h (which pulls in
 * dolphin/types.h). Only the typedefs lbtrigf.c and lb_00CE.c use.
 */
#ifndef MELEE_LB_REF_SHIM_PLATFORM_H
#define MELEE_LB_REF_SHIM_PLATFORM_H

#include <stdbool.h>
#include <stddef.h>

typedef signed char s8;
typedef unsigned char u8;
typedef signed short s16;
typedef unsigned short u16;
typedef int s32;
typedef unsigned int u32;
typedef long long s64;
typedef unsigned long long u64;
typedef float f32;
typedef double f64;

#endif
