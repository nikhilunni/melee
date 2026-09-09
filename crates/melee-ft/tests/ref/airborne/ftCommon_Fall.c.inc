void ftCommon_Fall(Fighter* fp, float gravity, float terminal_vel)
{
    fp->self_vel.y -= gravity;
    if (fp->self_vel.y < -terminal_vel) {
        fp->self_vel.y = -terminal_vel;
    }
}
