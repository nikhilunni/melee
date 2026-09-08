/* Shim for sysdolphin/baselib/debug.h: HSD_ASSERT aborts the oracle. */
#ifndef ANIM_REF_DEBUG_H
#define ANIM_REF_DEBUG_H

#include "platform_shim.h"

void anim_ref_assert_fail(const char* file, int line, const char* expr);

#define HSD_ASSERT(line, cond)                                                \
    ((cond) ? ((void) 0) : anim_ref_assert_fail(__FILE__, __LINE__, #cond))

#endif
