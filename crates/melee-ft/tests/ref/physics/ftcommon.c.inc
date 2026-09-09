void ftCommon_ApplyFrictionGround(Fighter* fp, float friction)
{
    if (ABS(friction) > ABS(fp->gr_vel)) {
        friction = -fp->gr_vel;
    } else if (fp->gr_vel > 0) {
        friction = -friction;
    }
    fp->xE4_ground_accel_1 = friction;
}

void ftCommon_ApplyGroundMovement(HSD_GObj* gobj)
{
    float temp_f1;
    Vec3* ground_normal;
    Fighter* fp = gobj->user_data;

    u8 _[8];

    ground_normal = &fp->coll_data.floor.normal;
    temp_f1 = ft_GetGroundFrictionMultiplier(fp);
    if (temp_f1 < 1) {
        fp->xE4_ground_accel_1 *= temp_f1;
    }
    fp->x74_anim_vel.x = +ground_normal->y * fp->xE4_ground_accel_1;
    fp->x74_anim_vel.y = -ground_normal->x * fp->xE4_ground_accel_1;
    fp->x74_anim_vel.z = 0;
    fp->self_vel.x = +ground_normal->y * fp->gr_vel;
    fp->self_vel.y = -ground_normal->x * fp->gr_vel;
    fp->self_vel.z = 0;
}

void ftCommon_8007CCA0(Fighter* fp, float arg1)
{
    if (fp->xF0_ground_kb_vel < 0) {
        fp->xF0_ground_kb_vel = fp->xF0_ground_kb_vel + arg1;
        if (fp->xF0_ground_kb_vel > 0) {
            fp->xF0_ground_kb_vel = 0;
        }
    } else {
        fp->xF0_ground_kb_vel = fp->xF0_ground_kb_vel - arg1;
        if (fp->xF0_ground_kb_vel < 0) {
            fp->xF0_ground_kb_vel = 0;
        }
    }
}
