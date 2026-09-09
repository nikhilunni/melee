            /* Apply stiffness blend toward natural direction */
            if (cur->desc.lb_unk0.unk_4C * desc->pos.x < 1.0) {
                f32 stiff_angle = ABS(lbVector_Angle(&link_dir, &current_dir));
                if (stiff_angle != 0.0f) {
                    PSVECCrossProduct(&link_dir, &current_dir,
                                      &stiffness_axis);
                    lbVector_Normalize(&stiffness_axis);
                    lbVector_RotateAboutUnitAxis(
                        &link_dir, &stiffness_axis,
                        stiff_angle *
                            (f32) (1.0 - (f64) (cur->desc.lb_unk0.unk_4C *
                                                desc->pos.x)));
                }
            }

            /* Apply gravity influence */
            {
                f32 grav_angle;
                grav_dir = lb_803B7280.v0;
                grav_angle = lbVector_Angle(&link_dir, &grav_dir);
                if (grav_angle != 0.0f) {
                    f32 grav_rot = cur->desc.lb_unk0.unk_8C * sinf(grav_angle);
                    if (grav_rot < 0.0f) {
                        grav_rot = -grav_rot;
                    }
                    {
                        PSVECCrossProduct(&link_dir, &grav_dir, &gravity_axis);
                        lbVector_Normalize(&gravity_axis);
                        lbVector_RotateAboutUnitAxis(&link_dir, &gravity_axis,
                                                     grav_rot);
                    }
                }
            }

            /* Apply interaction force */
            force_mag = 0.0f;
            if (lb_804D63B0 != NULL && first_active <= loop_index) {
                struct DynamicsData* next_node = cur->next;
                next_pos = next_node->desc.lb_unk0.unk_2C;
                force_mag = lb_800101C8(&next_pos, &force_dir);
                {
                    f32 force_angle = lbVector_Angle(&link_dir, &force_dir);
                    if (force_angle != 0.0f) {
                        f32 force_rot = (force_mag * sinf(force_angle)) /
                                        cur->desc.lb_unk0.unk_48;
                        if (force_rot < 0.0f) {
                            force_rot = -force_rot;
                        }
                        {
                            PSVECCrossProduct(&link_dir, &force_dir,
                                              &force_axis);
                            lbVector_Normalize(&force_axis);
                            lbVector_RotateAboutUnitAxis(
                                &link_dir, &force_axis, force_rot);
                        }
                    }
                }
            }

            /* Apply angular velocity rotation */
            if (0.0 != cur->desc.lb_unk0.unk_44) {
                lbVector_RotateAboutUnitAxis(&link_dir,
                                             &cur->desc.lb_unk0.unk_38,
                                             cur->desc.lb_unk0.unk_44);
            }

            /* Apply max angle constraint */
            if (force_mag <
                cur->desc.lb_unk0.unk_8C * cur->desc.lb_unk0.unk_48)
            {
                clamp_dir = saved_dir;
                if (lbVector_Angle(&clamp_dir, &link_dir) >
                    cur->desc.lb_unk0.unk_88)
                {
                    PSVECCrossProduct(&clamp_dir, &link_dir, &clamp_axis);
                    lbVector_Normalize(&clamp_axis);
                    lbVector_RotateAboutUnitAxis(&clamp_dir, &clamp_axis,
                                                 cur->desc.lb_unk0.unk_88);
                    link_dir = clamp_dir;
                }
            }

            /* Apply convergence limit */
            if (cur->desc.lb_unk0.unk_50 > 0.0) {
                if (lbVector_Angle(&natural_dir, &link_dir) <
                    cur->desc.lb_unk0.unk_50)
                {
                    link_dir = natural_dir;
                } else {
                    PSVECCrossProduct(&link_dir, &natural_dir,
                                      &convergence_axis);
                    lbVector_Normalize(&convergence_axis);
                    lbVector_RotateAboutUnitAxis(&link_dir, &convergence_axis,
                                                 cur->desc.lb_unk0.unk_50);
                }
            }

            /* Apply max angle deviation */
            {
                f32 dev_angle = lbVector_Angle(&natural_dir, &link_dir);
                if (dev_angle > cur->desc.lb_unk0.unk_68) {
                    PSVECCrossProduct(&link_dir, &natural_dir,
                                      &deviation_axis);
                    lbVector_Normalize(&deviation_axis);
                    lbVector_RotateAboutUnitAxis(&link_dir, &deviation_axis,
                                                 dev_angle -
                                                     cur->desc.lb_unk0.unk_68);
                }
            }

