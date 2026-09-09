void ftAnim_8006E9B4(Fighter_GObj* gobj)
{
    Fighter* fp;
    HSD_JObj* jobj;      // r27
    HSD_JObj* anim_jobj; // r27
    float framerate;     // f1
    float blend_t_inv;   // f31
    float blend_t;       // f30

    fp = GET_FIGHTER(gobj);

    if (fp->anim_id == -1) {
        return;
    }

    jobj = GET_JOBJ(gobj);
    if (fp->x8A4_animBlendFrames == 0.0F) {
        HSD_JObjClearFlagsAll(jobj, JOBJ_USE_QUATERNION);
        if (fp->x594_b0) {
            ftAnim_8006E054(
                fp, jobj,
                fp->parts[ftParts_GetBoneIndex(fp, FtPart_TransN)].joint,
                fp->parts[ftParts_GetBoneIndex(fp, 0x35)].joint);
        } else {
            ftAnim_8006E7B8(fp, FtPart_TopN);
        }
    } else {
        anim_jobj = fp->x8AC_animSkeleton;
        framerate = lbGetJObjFramerate(anim_jobj);
        fp->x8A8_anim_frame += framerate;
        if (fp->x8A4_animBlendFrames <= fp->x8A8_anim_frame) {
            fp->x8A8_anim_frame = fp->x8A4_animBlendFrames;
            blend_t = 1.0F;
            blend_t_inv = 0.0F;
        } else {
            float remaining; // f2
            remaining = fp->x8A4_animBlendFrames - fp->x8A8_anim_frame;
            blend_t = framerate / (framerate + remaining);
            blend_t_inv = 1.0F - blend_t;
        }
        ftAnim_8006E7B8(fp, FtPart_TopN);
        if (fp->x594_b0) {
            ftAnim_8006E054(
                fp, anim_jobj,
                fp->parts[ftParts_GetBoneIndex(fp, FtPart_TransN)].x4_jobj2,
                fp->parts[ftParts_GetBoneIndex(fp, 0x35)].joint);
        } else {
            HSD_JObjAnimAll(anim_jobj);
        }
        if (blend_t_inv) {
            ftAnim_8006FE9C(fp, FtPart_TransN, blend_t, blend_t_inv);
        } else {
            ftAnim_8006FF74(fp, FtPart_TransN);
        }
    }

    if (fp->x594_b2) {
        if (ftAnim_8006F3DC(gobj) < fp->cur_anim_frame) {
            fp->x898_unk += fp->cur_anim_frame + fp->frame_speed_mul;
        }
    }

    fp->cur_anim_frame = ftAnim_8006F3DC(gobj);
}
