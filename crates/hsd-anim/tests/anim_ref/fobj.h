/*
 * Shim for sysdolphin/baselib/fobj.h. The defines, HSD_FObj, HSD_FObjDesc,
 * HSD_ObjData and the prototypes are copied from the decomp header; the
 * allocator and forward-declaration headers are replaced by the minimal
 * types below (HSD_ObjAllocData is opaque to fobj.c beyond being passed to
 * HSD_ObjAllocInit / HSD_ObjAlloc / HSD_ObjFree, which driver.c stubs).
 */
#ifndef ANIM_REF_FOBJ_H
#define ANIM_REF_FOBJ_H

#include "platform_shim.h"

#define HSD_A_OP_NONE 0
#define HSD_A_OP_CON 1
#define HSD_A_OP_LIN 2
#define HSD_A_OP_SPL0 3
#define HSD_A_OP_SPL 4
#define HSD_A_OP_SLP 5
#define HSD_A_OP_KEY 6

#define HSD_A_FRAC_FLOAT (0 << 5)
#define HSD_A_FRAC_S16 (1 << 5)
#define HSD_A_FRAC_U16 (2 << 5)
#define HSD_A_FRAC_S8 (3 << 5)
#define HSD_A_FRAC_U8 (4 << 5)

#define FOBJ_LOAD_DATA0 1
#define FOBJ_LOAD_DATA 2
#define FOBJ_LOAD_WAIT 3

#define TYPE_ROBJ 1
#define TYPE_JOBJ 12

typedef struct _HSD_ObjAllocData {
    size_t size;
} HSD_ObjAllocData;

void HSD_ObjAllocInit(HSD_ObjAllocData* data, size_t size, size_t align);
void* HSD_ObjAlloc(HSD_ObjAllocData* data);
void HSD_ObjFree(HSD_ObjAllocData* data, void* obj);

typedef struct HSD_FObj HSD_FObj;

struct HSD_FObj {
    struct HSD_FObj* next;
    u8* ad;
    u8* ad_head;
    u32 length;
    u8 flags;
    u8 op;
    u8 op_intrp;
    u8 obj_type;
    u8 frac_value;
    u8 frac_slope;
    u16 nb_pack;
    s16 startframe;
    u16 fterm;
    f32 time;
    f32 p0;
    f32 p1;
    f32 d0;
    f32 d1;
};

typedef struct _HSD_FObjDesc {
    struct _HSD_FObjDesc* next;
    u32 length;
    f32 startframe;
    u8 type;
    u8 frac_value;
    u8 frac_slope;
    u8 dummy0;
    u8* ad;
} HSD_FObjDesc;

typedef union HSD_ObjData {
    f32 fv;
    s32 iv;
    Vec3 p;
} HSD_ObjData;

typedef void (*HSD_ObjUpdateFunc)(void* obj, enum_t type, HSD_ObjData* fval);

HSD_ObjAllocData* HSD_FObjGetAllocData(void);
void HSD_FObjInitAllocData(void);
void HSD_FObjRemove(HSD_FObj* fobj);
void HSD_FObjRemoveAll(HSD_FObj* fobj);
u32 HSD_FObjSetState(HSD_FObj* fobj, u32 state);
u32 HSD_FObjGetState(HSD_FObj* fobj);
void HSD_FObjReqAnimAll(HSD_FObj* fobj, f32 startframe);
void HSD_FObjStopAnim(HSD_FObj* fobj, void* obj, HSD_ObjUpdateFunc obj_update,
                      f32 rate);
void HSD_FObjStopAnimAll(HSD_FObj* fobj, void* obj,
                         HSD_ObjUpdateFunc obj_update, f32 rate);
void FObjUpdateAnim(HSD_FObj* fobj, void* obj, HSD_ObjUpdateFunc update_func);
void HSD_FObjInterpretAnim(HSD_FObj* fobj, void* obj,
                           HSD_ObjUpdateFunc obj_update, f32 rate);
void HSD_FObjInterpretAnimAll(void* fobj, void* obj,
                              HSD_ObjUpdateFunc obj_update, f32 rate);
HSD_FObj* HSD_FObjLoadDesc(HSD_FObjDesc* desc);
HSD_FObj* HSD_FObjAlloc(void);
void HSD_FObjFree(HSD_FObj* fobj);

#endif
