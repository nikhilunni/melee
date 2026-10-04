//! Motion helpers Ness's rows share (ft_081B.c, ft_084E.c, ftcommon.c).
use crate::init::Ness;
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::airborne,
};
use melee_it::{ItemControl, ItemRequest};
use melee_types::{CommonMotionState, GroundOrAir, ItemKind};

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
/// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27.
pub const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);
/// Ft_MF_KeepGfx.
pub const KEEP_GFX: u32 = 1 << 1;
/// Ft_MF_SkipHit.
pub const SKIP_HIT: u32 = 1 << 3;

/// A ported row of ftNs_Init_MotionStateTable; `animation` is the table's
/// anim_id column (0x803CC650).
pub const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The empty IASA callbacks.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// Fighter_ChangeMotionState(gobj, state, flags, start, rate, 0, NULL).
pub fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, flags, start, rate)
}

/// ft_8008A2BC: Wait.
pub fn wait(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(CommonMotionState::Wait.into(), assets)
}

/// ftCo_Fall_Enter: Fall, leaving the ground first when grounded.
pub fn fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        f.leave_ground();
    }
    f.change_motion_state(CommonMotionState::Fall.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision that walks off the
/// floor's edge. True while supported.
pub fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge. True while supported.
pub fn stays_on_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub fn lands(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
pub fn fall_at(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// `fp->parts[n]`: Ness's code indexes the parts array with the part enum
/// directly (no ftParts_GetBoneIndex).
pub fn part(part: melee_types::FtPart) -> usize {
    i32::from(part) as usize
}

/// Fighter_ChangeMotionState's efAsync_QueueFlush (fighter.c:951) from an
/// animation callback: what the outgoing script issued this frame spawns
/// before the change, ahead of the new script's frame-0 effects and colour
/// step.
pub fn seal_graphics(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    if !f.core.commands.graphics.is_empty() {
        f.core.resolve_graphics_commands(assets, rng);
    }
}

/// ft_PlaySFX(fp, id, 127, 64) for an id outside the footstep range (no
/// pitch draw).
pub fn play_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// An article of `kind` Ness owns is destroyed at once (the kinds'
/// Item_8026A8EC wrappers: it_802AD6B8 for the bat).
pub fn remove_article(f: &mut Fighter, kind: ItemKind) {
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind,
        control: ItemControl::Remove,
    });
}

/// take_dmg_cb and death2_cb = ftNs_Init_OnDamage, installed with an
/// article until the next motion change.
pub fn install_damage_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Ness>().damage_callbacks = true;
}
