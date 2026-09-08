/*
 * Host shim for sysdolphin/baselib/objalloc.h, plus the HSD_ASSERT macro
 * that mtx.c normally gets from debug.h (whose include guard the oracle
 * build predefines to keep the real header out).
 */
#ifndef HSD_ANIM_REF_MTX_OBJALLOC_H
#define HSD_ANIM_REF_MTX_OBJALLOC_H

#include <Runtime/platform.h>

typedef struct _HSD_ObjAllocData {
    u8 pad[0x2C];
} HSD_ObjAllocData;

void* HSD_ObjAlloc(HSD_ObjAllocData* data);
void HSD_ObjFree(HSD_ObjAllocData* data, void* obj);
void HSD_ObjAllocInit(HSD_ObjAllocData* data, u32 size, u32 align);

#define HSD_ASSERT(line, cond) ((void) (cond))

#endif
