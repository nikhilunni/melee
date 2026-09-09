/* Retail arithmetic excerpt adapter, ft/ftcommon.c (doldecomp/melee).
 * All four functions have no fused instructions (asm.py --fused).
 * Use -ffp-contract=off, including the drift scaling + base site. */
#include <stdio.h>
#include <stdbool.h>
#define ABS(x) ((x) < 0 ? -(x) : (x))
typedef struct { float x, y, z; } Vec3;
typedef struct {
    float air_drift_stick_mul, aerial_drift_base, air_drift_max;
    float aerial_friction, air_max_horizontal_velocity;
} ftCo_DatAttrs;
typedef struct {
    Vec3 self_vel, x74_anim_vel;
    struct { Vec3 lstick[1]; } input;
    ftCo_DatAttrs co_attrs;
} Fighter;
#include "ftCommon_ApplyFrictionAir.c.inc"
#include "ftCommon_8007D174.c.inc"
#include "ftCommon_8007D28C.c.inc"
#include "ftCommon_Fall.c.inc"
int main(void) {
    float in[10];
    while (fread(in, sizeof in, 1, stdin) == 1) {
        Fighter f = {0};
        f.self_vel.x = in[0]; f.self_vel.y = in[1];
        f.input.lstick[0].x = in[2];
        f.co_attrs = (ftCo_DatAttrs){in[5], in[6], in[7], in[8], in[9]};
        ftCommon_Fall(&f, in[3], in[4]);
        ftCommon_8007D28C(&f, f.self_vel.x);
        float out[3] = {f.self_vel.y, f.x74_anim_vel.x,
                        f.self_vel.x + f.x74_anim_vel.x};
        if (fwrite(out, sizeof out, 1, stdout) != 1) return 1;
    }
    return ferror(stdin) ? 1 : 0;
}
