void ftCommon_ApplyFrictionAir(Fighter* fp, float friction)
{
    if (ABS(friction) >= ABS(fp->self_vel.x)) {
        friction = -fp->self_vel.x;
    } else if (fp->self_vel.x > 0) {
        friction = -friction;
    }
    fp->x74_anim_vel.x = friction;
}
