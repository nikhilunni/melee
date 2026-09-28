//! The transformation (ftCommon_8007EFC8): a character with two forms
//! (Zelda and Sheik) hands the match from the form in play to the other,
//! which slept beside it since Player_80031AD0 created both.
//!
//! The form in play raises the request from its accessory4 once its
//! transformation motion ends (ftZd_SpecialLw_8013AEAC, Sheik's
//! fn_8011412C); the scene, which owns both fighters, then calls
//! [`transfer`] and makes the partner the player's fighter.
use super::{
    assets::{FighterAssets, Result},
    Fighter, SpawnCounter,
};
use hsd_types::Vec3;
use melee_cm::StageCamera;
use melee_mp::CollMap;

impl super::FighterCore {
    /// ftCommon_8007EFC8's caller: the scene transforms this player after
    /// the proc.
    pub fn request_transformation(&mut self) {
        self.transformation_requested = true;
    }
    /// Take the pending request (the scene performs it once).
    pub fn take_transformation_request(&mut self) -> bool {
        std::mem::take(&mut self.transformation_requested)
    }
}

/// The scene's services the handover needs.
pub struct TransferContext<'a> {
    pub map: &'a CollMap,
    pub counter: &'a mut SpawnCounter,
    pub stage_camera: &'a StageCamera,
}

/// ftCommon_8007EFC8 (8007EFC8): `dst`, the sleeping partner, takes over
/// from `src`, the form in play, which goes to sleep; then the partner's
/// arrival (the callback `src` passes: the other form's finishing motion).
pub fn transfer(
    src: &mut Fighter,
    dst: &mut Fighter,
    src_assets: &FighterAssets,
    dst_assets: &FighterAssets,
    context: TransferContext<'_>,
) -> Result<()> {
    if src.core.held_item.is_some() {
        unimplemented!("ftCommon_8007EFC8: transforming with a held item (ftcommon.c:1360-1365)");
    }
    // ftcommon.c:1249-1252: the forms swap x221F_b4, so the one in play is
    // the player's own fighter (Player_SwapTransformedStates follows it).
    // Fighter_UnkInitReset_80067C98 reads the Player's coordinates, facing
    // and damage, which mirror the form in play.
    let dst_secondary = dst.core.player.secondary;
    dst.core.player = src.core.player.clone();
    src.core.player.secondary = dst_secondary;
    dst.core.player.position = src.core.player_position;
    dst.core.player.facing = src.core.player_facing;
    dst.core.player.damage = src.core.physics.percent;
    dst.core.spawn_number = context.counter.allocate();
    dst.core.reset_life(dst_assets, context.map);
    clear_velocities(dst);
    copy_state(src, dst);
    // ftcommon.c:1318-1324: src's x221D_b6 (its color overlay carried
    // over) is not modelled; otherwise ftCo_800C0200(dst, 9).
    dst.core
        .release_charge_color(INVINCIBILITY_FLASH, dst_assets);
    // `if ((src->x198C = 2) && src->x1990 != 0)`: the assignment is always
    // true, so a live ledge intangibility carries over (ftColl_8007B760).
    if src.core.status.ledge_intangibility != 0 {
        dst.core.status.ledge_intangibility = dst
            .core
            .status
            .ledge_intangibility
            .max(src.core.status.ledge_intangibility);
        dst.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: INVINCIBILITY_FLASH,
                duration: 0,
            });
    }
    // ftLib_SetScale, ftCo_800D105C and ft_80081C88 rescale from the same
    // model scale; metal, bunny hood and flower hats are item effects.
    // ftCamera_80076064: the partner's camera subject becomes active.
    dst.core
        .reset_camera_subject(dst_assets, context.stage_camera);
    // ft_800849EC: mpCopyCollData(src, dst, 2).
    melee_mp::copy_coll_data(&src.core.collision.data, &mut dst.core.collision.data, 2);
    // un_80322314 only raises the crowd's gasp.
    src.enter_sleep(src_assets)?;
    (dst.character.table().transformation_arrival)(dst, dst_assets)
}

/// ftCo_800C0200's argument: the intangibility flash's color animation.
const INVINCIBILITY_FLASH: u8 = 9;

/// _func_8007E2FC_inline: every velocity owner, the ground accelerations
/// included.
fn clear_velocities(f: &mut Fighter) {
    let physics = &mut f.core.physics;
    physics.ground_acceleration = 0.0;
    physics.secondary_ground_acceleration = 0.0;
    physics.animation_velocity = Vec3::ZERO;
    physics.ground_velocity = 0.0;
    physics.self_velocity = Vec3::ZERO;
    physics.ground_knockback_velocity = 0.0;
    physics.knockback_velocity = Vec3::ZERO;
    physics.ground_shield_knockback_velocity = 0.0;
    physics.shield_knockback_velocity = Vec3::ZERO;
}

/// ftcommon.c:1253-1317: the state the partner takes from the form in play.
fn copy_state(src: &Fighter, dst: &mut Fighter) {
    let (s, d) = (&src.core, &mut dst.core);
    d.physics.position = s.physics.position;
    d.physics.previous_position = s.physics.previous_position;
    d.physics.position_delta = s.physics.position_delta;
    d.physics.facing = s.physics.facing;
    // Player_SetHPByIndex mirrors it for the HUD.
    d.physics.percent = s.physics.percent;
    d.physics.self_velocity = s.physics.self_velocity;
    d.physics.ground_or_air = s.physics.ground_or_air;
    d.physics.ground_velocity = s.physics.ground_velocity;
    d.physics.ground_knockback_velocity = s.physics.ground_knockback_velocity;
    d.physics.ground_shield_knockback_velocity = s.physics.ground_shield_knockback_velocity;
    d.physics.player_nudge = s.physics.player_nudge;
    // fp->input and the analog/button timers x670..x68B.
    d.input = s.input.clone();
    d.physics.jumps_used = s.physics.jumps_used;
    d.status.wall_jump.used = s.status.wall_jump.used;
    d.status.shield_health = s.status.shield_health;
    // dmg.x1910: the magnifier's tick count.
    d.offscreen.magnified_ticks = s.offscreen.magnified_ticks;
    // The Player's stale-move queue and standing are the player's, not the
    // form's.
    d.combat.stale = s.combat.stale.clone();
    d.standing_rank = s.standing_rank;
    d.grab_handicap = s.grab_handicap;
    d.player_position = s.player_position;
    d.player_facing = s.player_facing;
}
