//! Dr. Mario's Megavitamins: the neutral special's thrown pill
//! (ftMr_SpecialN_ItemFireSpawn's FTKIND_DRMARIO branch) and the side taunt
//! that holds one (ftdrmarioappeals.c, ftDr_Init_801497CC..80149910).
use crate::init::DrMario;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        caches::part_position,
        state::{callbacks, AnimationPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// ftMr_MS_AppealSR / ftMr_MS_AppealSL.
pub const TAUNT_RIGHT: ActionId = ActionId(341);
pub const TAUNT_LEFT: ActionId = ActionId(342);

/// The script's cmd_vars[0]: 1 (set on entry) spawns the pill, 2 puts it
/// away.
const SPAWN_PILL: u32 = 1;
const PUT_AWAY_PILL: u32 = 2;

/// Fighter +2240 (the taunt's pill) and the callbacks it installed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TauntPill {
    /// +2240 is set: the pill exists.
    pub present: bool,
    /// take_dmg_cb / death2_cb = ftDr_Init_80149540.
    pub damage_callbacks: bool,
}

/// ftDr_Init_MotionStateTable rows 341/342: ftCo_SM_AppealSR/SL with
/// ftDr_AppealS_Anim, ftCo_AppealS_IASA, ft_80084F3C and ft_80084280.
pub const TAUNT_ROWS: [MotionRow; 2] = [taunt_row(TAUNT_RIGHT, 239), taunt_row(TAUNT_LEFT, 240)];

const fn taunt_row(action: ActionId, animation: i32) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim: taunt_animation,
        iasa: callbacks::input::appeal,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

fn taunt(f: &mut Fighter) -> &mut TauntPill {
    &mut f.character.get_mut::<DrMario>().taunt
}

/// ftDr_Init_80149910 (80149910): ftCo_800DEAE8 enters the right or left
/// row, then cmd_vars[0] = 1 and cmd_vars[1] = 0.
pub fn enter_taunt(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.allow_interrupt = false;
    let state = if f.physics.facing == -1.0 && assets.left_taunt_available {
        TAUNT_LEFT
    } else {
        TAUNT_RIGHT
    };
    f.change_motion_state(state, assets)?;
    f.commands.variables[0] = SPAWN_PILL;
    f.commands.variables[1] = 0;
    Ok(())
}

/// ftDr_AppealS_Anim (80149954): the pill at the root on cmd_vars[0] = 1
/// (itDrMarioPill_Appeal_Spawn, with ftDr_Init_80149540 as the damage
/// callbacks), put away on 2 (ftDr_Init_801497CC) and at the animation's
/// end, which returns to Wait (ft_8008A2BC).
fn taunt_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == SPAWN_PILL && !taunt(f).present {
        // lb_8000B1CC(fp->parts->joint, NULL, &pos).
        let root = usize::from(p.assets.parts.joint(FtPart::TopN).expect("TopN part"));
        let c = &mut f.core;
        let position = part_position(&mut c.skeleton, &c.animation, root, Vec3::ZERO);
        let colour = ft_mario_family::vitamin_random::<DrMario>(f, p.rng);
        spawn(f, position, it_drmariopill::taunt_argument(colour));
        let t = taunt(f);
        t.present = true;
        t.damage_callbacks = true;
    } else if f.commands.variables[0] == PUT_AWAY_PILL {
        put_away(f);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        put_away(f);
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftDr_Init_801497CC (801497CC): a pill is destroyed
/// (itDrMarioPill_802C0DBC) and the damage callbacks go.
fn put_away(f: &mut Fighter) {
    if std::mem::take(&mut taunt(f).present) {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::DrMarioVitamin,
            control: ItemControl::Remove,
        });
    }
    taunt(f).damage_callbacks = false;
}

/// ftDr_Init_80149540: the taunt's take-damage and death2 callback.
pub fn taunt_damage_callback(f: &mut Fighter) {
    if taunt(f).damage_callbacks {
        put_away(f);
    }
}

/// ftDr_Init_801498A0 (801498A0): the taunt pill let go of its owner
/// (itDrMarioPill_Motion2_Anim), clearing +2240 and the callbacks.
pub fn released(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::DrMarioVitamin {
        let t = taunt(f);
        t.present = false;
        t.damage_callbacks = false;
    }
}

/// ftMr_SpecialN_ItemFireSpawn's Dr. Mario branch: a colour
/// (ftMr_SpecialN_VitaminRandom), then itDrMarioPill_Spawn at the hand.
pub fn throw(
    f: &mut Fighter,
    _assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
    hand: Vec3,
    _bone: usize,
) {
    let colour = ft_mario_family::vitamin_random::<DrMario>(f, rng);
    spawn(f, hand, it_drmariopill::thrown_argument(colour));
}

/// itDrMarioPill_Spawn / itDrMarioPill_Appeal_Spawn (802C0510 / 802C0850):
/// prev_pos is `position` on the stage plane; pos is it_8026BB68's ECB
/// midpoint (ftLib_80086990, retail 800869AC..BC: fadds, fmuls, fadds).
fn spawn(f: &mut Fighter, position: Vec3, argument: i32) {
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::DrMarioVitamin,
        c.player.id,
        position,
        c.physics.facing,
    );
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = argument;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}
