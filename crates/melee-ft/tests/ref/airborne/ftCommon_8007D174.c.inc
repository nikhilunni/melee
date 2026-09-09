void ftCommon_8007D174(Fighter* fp, float vel, float accel, float target_vel,
                       float friction)
{
    if (!target_vel) {
        ftCommon_ApplyFrictionAir(fp, friction);
        return;
    }

    if (!(vel * accel < 0.0F)) {
        if (accel > 0) {
            if (vel + accel > target_vel) {
                accel = -friction;
                if (vel + accel < target_vel) {
                    accel = target_vel - vel;
                }
                if (vel + accel > fp->co_attrs.air_max_horizontal_velocity) {
                    accel = fp->co_attrs.air_max_horizontal_velocity - vel;
                }
            }
        } else if (vel + accel < target_vel) {
            accel = friction;
            if (vel + accel > target_vel) {
                accel = target_vel - vel;
            }
            if (vel + accel < -fp->co_attrs.air_max_horizontal_velocity) {
                accel = -fp->co_attrs.air_max_horizontal_velocity - vel;
            }
        }
    }
    fp->x74_anim_vel.x = accel;
}
