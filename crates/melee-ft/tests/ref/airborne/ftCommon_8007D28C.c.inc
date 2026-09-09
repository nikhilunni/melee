void ftCommon_8007D28C(Fighter* fp, float vel)
{
    float accel_scaling;
    float accel_flat;
    float lsx = fp->input.lstick[0].x;
    ftCo_DatAttrs* attrs = &fp->co_attrs;
    accel_scaling = lsx * attrs->air_drift_stick_mul;

    if (lsx > 0) {
        accel_flat = +attrs->aerial_drift_base;
    } else {
        accel_flat = -attrs->aerial_drift_base;
    }
    ftCommon_8007D174(fp, vel, accel_scaling + accel_flat,
                      lsx * attrs->air_drift_max, attrs->aerial_friction);
}
