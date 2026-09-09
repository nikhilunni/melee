/* Arithmetic oracle: verbatim decomp function bodies, scalar-only callees.
 * asm.py ftAnim_8006E9B4 HSD_AObjInterpretAnim --fused: no fused sites.
 * E9B4 0x8006EA70 / 0x8006EA9C..EAB0 and 0x8006EB70/EB74 are f32.
 * FObjs/SRT, callback dispatch and root extraction do not change these clocks.
 */
#include <stdint.h>
#include <stdio.h>
#include <math.h>
#include <stdbool.h>
#include <string.h>
typedef float f32;
typedef void (*HSD_ObjUpdateFunc)(void);
#define AOBJ_REWINDED (1u << 26)
#define AOBJ_FIRST_PLAY (1u << 27)
#define AOBJ_NO_UPDATE (1u << 28)
#define AOBJ_LOOP (1u << 29)
#define AOBJ_NO_ANIM (1u << 30)
typedef struct { uint32_t flags; float curr_frame, framerate, end_frame, rewind_frame; void *fobj; } HSD_AObj;
static int HSD_AObj_804D762C, HSD_AObj_804D7630;
static void HSD_FObjStopAnimAll(void *f, void *o, HSD_ObjUpdateFunc u, float r) {}
static void HSD_FObjReqAnimAll(void *f, float r) {}
static void HSD_FObjInterpretAnimAll(void *f, void *o, HSD_ObjUpdateFunc u, float r) {}
#include "aobj.c.inc"
typedef struct { HSD_AObj *aobj; } HSD_JObj;
typedef struct { HSD_JObj *joint, *x4_jobj2; } Part;
typedef struct {
    int anim_id;
    float cur_anim_frame, frame_speed_mul, x898_unk, x8A4_animBlendFrames, x8A8_anim_frame;
    bool x594_b0, x594_b2;
    HSD_JObj *x8AC_animSkeleton;
    Part parts[2];
} Fighter;
typedef struct { Fighter *fp; HSD_JObj *root; } Fighter_GObj;
#define GET_FIGHTER(g) ((g)->fp)
#define GET_JOBJ(g) ((g)->root)
#define FtPart_TopN 0
#define FtPart_TransN 1
#define JOBJ_USE_QUATERNION 0x20000
static int ftParts_GetBoneIndex(Fighter *f, int p) { return 1; }
static void HSD_JObjClearFlagsAll(HSD_JObj *j, int f) {}
static void HSD_JObjAnimAll(HSD_JObj *j) { HSD_AObjInterpretAnim(j->aobj, NULL, NULL); }
static void ftAnim_8006E054(Fighter *f, HSD_JObj *j, HSD_JObj *a, HSD_JObj *b) { HSD_JObjAnimAll(j); }
static void ftAnim_8006E7B8(Fighter *f, int p) { HSD_JObjAnimAll(f->parts[0].joint); }
static float lbGetJObjFramerate(HSD_JObj *j) { return j->aobj->framerate; }
static void ftAnim_8006FE9C(Fighter *f, int p, float a, float b) {}
static void ftAnim_8006FF74(Fighter *f, int p) {}
static float ftAnim_8006F3DC(Fighter_GObj *g) {
    Fighter *f = g->fp;
    return (f->x8A4_animBlendFrames == 0.0f ? g->root : f->x8AC_animSkeleton)->aobj->curr_frame;
}
#include "ftanim.c.inc"
int main(void) {
    uint32_t input[9];
    while (fread(input, sizeof(input), 1, stdin) == 1) {
        float values[7]; memcpy(values, input, sizeof(values));
        HSD_AObj a = { .flags=input[7], .curr_frame=values[0], .framerate=values[1], .end_frame=values[2], .rewind_frame=values[3] };
        HSD_JObj joint = { &a };
        /* The main tree has no AObj during blending; avoid interpreting the
           active blend AObj twice in the two-tree E9B4 path. */
        HSD_AObj stopped = { .flags=AOBJ_NO_ANIM };
        HSD_JObj old = { &stopped };
        Fighter f = { .anim_id=2, .cur_anim_frame=values[0], .frame_speed_mul=values[1], .x898_unk=values[4],
            .x8A4_animBlendFrames=values[5], .x8A8_anim_frame=values[6], .x594_b2=input[8], .x8AC_animSkeleton=&joint };
        f.parts[0].joint = values[5] == 0.0f ? &joint : &old;
        Fighter_GObj g = { &f, f.parts[0].joint };
        ftAnim_8006E9B4(&g);
        float results[3] = { f.cur_anim_frame, f.x898_unk, f.x8A8_anim_frame };
        fwrite(results, sizeof(results), 1, stdout);
        fwrite(&a.flags, sizeof(a.flags), 1, stdout);
    }
    return ferror(stdin) || ferror(stdout);
}
