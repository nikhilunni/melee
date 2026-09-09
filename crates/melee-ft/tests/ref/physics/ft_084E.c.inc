void ft_80084F3C(Fighter_GObj* gobj)
{
    Fighter* fp = GET_FIGHTER(gobj);
    ftCo_DatAttrs* co = &fp->co_attrs;
    f32 friction = co->ground_friction;
    PAD_STACK(8);
    if (ABS(fp->gr_vel) > co->walk_max_vel) {
        friction *= p_ftCommonData->friction_when_above_walk_speed;
    }
    ftCommon_ApplyFrictionGround(fp, friction);
    ftCommon_ApplyGroundMovement(gobj);
}
