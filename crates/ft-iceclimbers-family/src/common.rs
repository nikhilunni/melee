//! Motion helpers the Ice Climbers' specials and Kirby's copied Ice Shot
//! share (ft_081B.c, ft_0877.c, ft_0892.c).
use melee_ft::fighter::{assets::FighterAssets, Fighter};

/// No IASA (ftPp_SpecialN_IASA and siblings are empty).
pub fn no_input(_: &mut Fighter, _: melee_ft::fighter::state::InputPhase<'_>) {}

/// ft_PlaySFX(fp, id, 127, 64).
pub fn play_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ft_800881D8(fp, id, 127, 64): the climber's voice.
pub fn play_voice(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::FighterVoice,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ft_8008A2BC from a grounded row, ftCo_Fall_Enter from an aerial one.
///
/// A grounded row is not always on the ground: the partner joins the
/// Blizzard through its grounded entry whatever her footing
/// (ftCo_800B0AF4), and ftPp_SpecialLw_Coll keeps that row while the floor
/// she last stood on still supports her. ft_8008A348 (8008A348) then lands
/// her first (ftCommon_8007D7FC) and enters Wait.
pub fn finish(
    f: &mut Fighter,
    assets: &FighterAssets,
    air: bool,
) -> melee_ft::fighter::assets::Result<()> {
    if air {
        return f.change_motion_state(melee_types::CommonMotionState::Fall.into(), assets);
    }
    if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        f.core.land();
    }
    f.change_motion_state(melee_types::CommonMotionState::Wait.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision that lets the fighter
/// walk off the floor's edge. True while it is still supported.
pub fn stays_grounded(
    f: &mut Fighter,
    p: &mut melee_ft::fighter::state::CollisionPhase<'_>,
) -> bool {
    use melee_ft::collision::ground;
    let c = &mut f.core;
    matches!(
        ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ),
        ground::WaitGroundResult::Supported
    )
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub fn lands(f: &mut Fighter, p: &mut melee_ft::fighter::state::CollisionPhase<'_>) -> bool {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    )
}
