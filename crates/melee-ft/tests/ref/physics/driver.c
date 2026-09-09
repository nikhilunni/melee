// C oracle: verbatim ground functions plus the retail procUpdate arithmetic.
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
typedef uint8_t u8;
typedef float f32;
typedef struct { float x,y,z; } Vec3;
typedef struct { void *user_data; } HSD_GObj;
typedef HSD_GObj Fighter_GObj;
struct ftCo_DatAttrs { float ground_friction, walk_max_vel; };
typedef struct ftCo_DatAttrs ftCo_DatAttrs;
typedef struct {
    float gr_vel, xE4_ground_accel_1, xE8_ground_accel_2, xF0_ground_kb_vel;
    Vec3 x74_anim_vel, self_vel, cur_pos, kb, shield, nudge, floor_speed, wind, origin;
    struct { struct { Vec3 normal; } floor; } coll_data;
    ftCo_DatAttrs co_attrs;
    float terrain;
} Fighter;
struct { float friction_when_above_walk_speed; } common;
#define p_ftCommonData (&common)
#define GET_FIGHTER(g) ((Fighter*)(g)->user_data)
#define PAD_STACK(n)
#define ABS(x) ((x)<0?-(x):(x))
float ft_GetGroundFrictionMultiplier(Fighter *fp) { return fp->terrain; }
#include "ftcommon.c.inc"
#include "ft_084E.c.inc"
static Vec3 add(Vec3 a, Vec3 b) { return (Vec3){a.x+b.x,a.y+b.y,a.z+b.z}; }
static Vec3 vector(const float *f) { return (Vec3){f[0],f[1],f[2]}; }
static void emit(float x) { fwrite(&x,4,1,stdout); }
static void emit_vec(Vec3 v) { emit(v.x);emit(v.y);emit(v.z); }
int main(void) {
    float in[36];
    while(fread(in,sizeof in,1,stdin)==1) {
        Fighter fp={0}; HSD_GObj g={&fp};
        fp.gr_vel=in[0]; fp.co_attrs.ground_friction=in[1]; fp.co_attrs.walk_max_vel=in[2];
        common.friction_when_above_walk_speed=in[3]; fp.terrain=in[4];
        fp.coll_data.floor.normal=vector(in+5); fp.cur_pos=vector(in+8);
        fp.kb=vector(in+11); fp.shield=vector(in+14); fp.nudge=vector(in+17);
        fp.floor_speed=vector(in+20); fp.wind=vector(in+23); fp.origin=vector(in+26);
        int duration=(int)in[29], remaining=(int)in[30];
        fp.xE8_ground_accel_2=in[31];
        ft_80084F3C(&g);
        emit(fp.xE4_ground_accel_1); emit_vec(fp.x74_anim_vel); emit_vec(fp.self_vel);
        // fighter.c:2292..2301; retail 0x8006BBDC/BBE4 are two fadds.
        fp.gr_vel += fp.xE4_ground_accel_1 + fp.xE8_ground_accel_2;
        fp.xE4_ground_accel_1 = fp.xE8_ground_accel_2 = 0;
        fp.self_vel = add(fp.self_vel,fp.x74_anim_vel);
        fp.x74_anim_vel=(Vec3){0};
        Vec3 velocity=fp.self_vel;
        if(duration!=0) {
            float progress=1.0f-(float)remaining/(float)duration;
#ifdef UNFUSED
            velocity.x=progress*(fp.self_vel.x-fp.origin.x)+fp.origin.x;
            velocity.y=progress*(fp.self_vel.y-fp.origin.y)+fp.origin.y;
#else
            // retail 0x8006BC7C/BC90: fmadds; no host implicit contraction.
            velocity.x=fmaf(progress,fp.self_vel.x-fp.origin.x,fp.origin.x);
            velocity.y=fmaf(progress,fp.self_vel.y-fp.origin.y,fp.origin.y);
#endif
            --remaining;
            if(remaining==0) duration=0;
        }
        // retail 0x8006BCC0/BCD0: no C +=0 y instruction; 0x8006BDA4..BDD4
        // likewise omits kb z +=0. KB z is deliberately nonzero in the sweep.
        fp.cur_pos.x+=fp.nudge.x; fp.cur_pos.z+=fp.nudge.z;
        fp.cur_pos=add(fp.cur_pos,velocity);
        fp.cur_pos.x+=fp.kb.x; fp.cur_pos.y+=fp.kb.y;
        fp.cur_pos=add(fp.cur_pos,fp.shield);
        if(in[32]!=0) fp.cur_pos=add(fp.cur_pos,fp.floor_speed);
        fp.cur_pos=add(fp.cur_pos,fp.wind);
        emit(fp.gr_vel); emit_vec(fp.self_vel); emit_vec(fp.cur_pos);
        emit((float)duration);emit((float)remaining);
        fp.xF0_ground_kb_vel=in[33];
        ftCommon_8007CCA0(&fp,in[34]);
        emit(fp.xF0_ground_kb_vel);
    }
    return ferror(stdin)||ferror(stdout);
}
