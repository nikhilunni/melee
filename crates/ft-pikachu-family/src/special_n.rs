//! Thunder Jolt, ftpikachuspecialn.c (80124868..80124C8C): the script's
//! cmd_vars[0] cue spawns the ball (it-pikachu) once per jolt
//! (cmd_vars[1]).
use crate::{
    common::{self, change, no_input},
    flags::{GROUND_AIR, KEEP_GFX},
    row, FamilyState as S, PikachuFamily,
};
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        commands::{FootstepSound, SoundChannel},
        state::{callbacks, AnimationPhase, CollisionPhase},
        Fighter, MotionRow,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftPk_MF_SpecialN_Coll: the counterparts keep the effects.
const GROUND_AIR_FLAGS: u32 = GROUND_AIR | KEEP_GFX;

pub const fn rows<C: PikachuFamily>() -> [MotionRow; 2] {
    [
        row(
            S::SpecialN,
            anim::<C, false>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision,
        ),
        row(
            S::SpecialAirN,
            anim::<C, true>,
            no_input,
            callbacks::physics::fall,
            air_collision,
        ),
    ]
}

/// ftPk_SpecialN_Enter / ftPk_SpecialAirN_Enter -> doEnter (80124868):
/// the motion, the four command variables, then ftAnim_8006EBA4.
pub fn enter<C: PikachuFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let state = if air { S::SpecialAirN } else { S::SpecialN };
    change(f, state.action(), 0, 0.0, 1.0, assets).expect("Thunder Jolt assets");
    f.commands.variables[..4].fill(0);
    f.step_animation(assets);
}

/// ftPk_SpecialN_Anim / ftPk_SpecialAirN_Anim (80124908 / 80124A18): the
/// first cue spawns the ball at the offset (x by the facing, both scaled
/// by the player's y scale; retail 80124970 / 80124984 fmadds). At the
/// end, Wait; in the air Fall, or a special fall with the landing lag.
fn anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == 1 {
        f.commands.variables[0] = 0;
        if f.commands.variables[1] == 0 {
            f.commands.variables[1] = 1;
            spawn_ball::<C, AIR>(f);
        }
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        if !AIR {
            return common::finish(f, false, p.assets).map(|()| None);
        }
        let lag = f
            .character
            .get::<C>()
            .attributes()
            .thunder_jolt
            .air_landing_lag;
        if lag == 0.0 {
            common::finish(f, true, p.assets)?;
        } else {
            // ftCo_80096900(gobj, 1, 0, 1, 1.0, lag).
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// itPikachuThunderJolt_Spawn (802B338C): the ball starts its sweep at
/// Pikachu's ECB centre (it_8026BB68 -> ftLib_80086990; retail 800869AC..
/// BC fadds, fmuls, fadds) toward the offset point, with an initial
/// collision; both jolts spawn the ground kind (specialn_itkind).
fn spawn_ball<C: PikachuFamily, const AIR: bool>(f: &mut Fighter) {
    let a = &f.character.get::<C>().attributes().thunder_jolt;
    let (offset, kind) = (
        if AIR {
            a.air_spawn_offset
        } else {
            a.ground_spawn_offset
        },
        a.ground_item,
    );
    let kind = ItemKind::try_from(kind as i32).expect("Thunder Jolt item kind");
    let scale = f.player.scale;
    let facing = f.physics.facing;
    let position = f.physics.position;
    let point = Vec3::new(
        fmadds(scale, offset.x * facing, position.x),
        fmadds(offset.y, scale, position.y),
        0.0,
    );
    let mut spawn = SpawnItem::ray(kind, f.player.id, point, facing);
    spawn.previous_position = point;
    let ecb = &f.collision.data.ecb;
    let midpoint = 0.5 * (ecb.top.y + ecb.bottom.y);
    spawn.position = Vec3::new(position.x + 0.0, position.y + midpoint, position.z + 0.0);
    f.core.item_requests.push(ItemRequest::Spawn(spawn));
    // ft_PlaySFX(fp, 240076, 127, 64) for Pikachu, 230067 for Pichu.
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id: C::JOLT_SOUND,
        volume: 127,
        pan: 64,
    });
}

/// ftPk_SpecialN_Coll: off the floor, the aerial jolt at the same frame.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Thunder Jolt collision assets");
        common::ground_to_air(f, S::SpecialAirN.action(), GROUND_AIR_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirN_Coll: landing (ftCommon_8007D7FC) stops the fall and
/// continues on the ground.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        f.land();
        f.physics.self_velocity.y = 0.0;
        let frame = f.animation.frame;
        let assets = p.assets.expect("Thunder Jolt landing assets");
        change(
            f,
            S::SpecialN.action(),
            GROUND_AIR_FLAGS,
            frame,
            1.0,
            assets,
        )?;
    }
    Ok(())
}

/// ftPk_SpecialN_Anim's sound for the kind.
pub const PIKACHU_JOLT_SOUND: u32 = 240076;
