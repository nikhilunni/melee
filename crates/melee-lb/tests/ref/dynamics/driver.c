/* Independent decomp excerpts with retail fused helper sites. See NOTICE. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../lbtrigf/retail/lbtrigf.c"
float MSL_TrigF_80400770[1];
float MSL_TrigF_80400774[1];
float sinf(float);
void __sinit_trigf_c(void);
#define M_TAU (2.0 * M_PI)
#define ABS(x) ((x) < 0.0f ? -(x) : (x))
typedef struct {float x,y,z;} Vec3;
static float lbVector_Len(Vec3* v) {return sqrtf(v->x*v->x+v->y*v->y+v->z*v->z);}
#include "retail_vector.c.inc"
/* PSVEC cross: SDK paired-single multiply then fused subtract. */
static void PSVECCrossProduct(Vec3* a,Vec3* b,Vec3* out) {
    out->x=gekko_fmsubs(a->y,b->z,a->z*b->y);
    out->y=-gekko_fmsubs(a->x,b->z,b->x*a->z);
    out->z=-gekko_fmsubs(a->y,b->x,b->y*a->x);
}
struct DynamicsData {
    struct {struct {
        Vec3 unk_2C,unk_38;
        float unk_44,unk_48,unk_4C,unk_50,unk_68,unk_88,unk_8C;
    } lb_unk0;} desc;
    struct DynamicsData* next;
};
struct Desc {Vec3 pos;};
static void* lb_804D63B0=NULL;
static struct {Vec3 v0;} lb_803B7280={{0,-1,0}};
/* The sweep exercises the no-external-force step. This branch is unreachable. */
static float lb_800101C8(Vec3* a,Vec3* b) {(void)a;(void)b;abort();}
static Vec3 step(float* in) {
    struct DynamicsData node={0},next={0};
    struct DynamicsData* cur=&node;
    struct Desc descriptor={{in[0],0,0}},*desc=&descriptor;
    node.next=&next;
    node.desc.lb_unk0.unk_4C=in[1]; node.desc.lb_unk0.unk_50=in[2];
    node.desc.lb_unk0.unk_68=in[3]; node.desc.lb_unk0.unk_88=in[4];
    node.desc.lb_unk0.unk_8C=in[5]; node.desc.lb_unk0.unk_44=in[6];
    node.desc.lb_unk0.unk_48=in[7];
    node.desc.lb_unk0.unk_38=(Vec3){in[17],in[18],in[19]};
    Vec3 natural_dir={in[8],in[9],in[10]},current_dir={in[11],in[12],in[13]};
    Vec3 saved_dir={in[14],in[15],in[16]},link_dir=saved_dir;
    Vec3 stiffness_axis,grav_dir,gravity_axis,next_pos,force_dir,force_axis;
    Vec3 clamp_dir,clamp_axis,convergence_axis,deviation_axis;
    float force_mag;
    int first_active=0,loop_index=0;
#include "spring_step.c.inc"
    return link_dir;
}
int main(int argc,char** argv) {
    if(argc!=3)return 2;
    __sinit_trigf_c();
    uint32_t nan=0x7fffffff,inf=0x7f800000;
    memcpy(MSL_TrigF_80400770,&nan,4); memcpy(MSL_TrigF_80400774,&inf,4);
    FILE* input=fopen(argv[1],"rb"),*output=fopen(argv[2],"wb");
    if(!input||!output)return 2;
    float values[20];
    while(fread(values,sizeof(values),1,input)==1) {Vec3 result=step(values);fwrite(&result,sizeof(result),1,output);}
    fclose(input);fclose(output);return 0;
}
