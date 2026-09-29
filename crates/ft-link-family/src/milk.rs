//! Young Link's side taunt, ftclinkappeals.c and ftclink.c
//! (80149268..80149318): the script's cmd_vars[1] = 1 puts a bottle of milk
//! in his left hand (it_802C8B28) and 2 puts it away, as does the taunt's
//! end, a hit or a KO (ftLk_800EAF58 -> ftCl_Init_80149268).
use crate::{FamilyState, LinkFamily};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, MotionRow},
        Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// The script's cmd_vars[1]: 1 takes the milk out, 2 puts it away.
const TAKE_OUT: u32 = 1;
const PUT_AWAY: u32 = 2;

/// ftCl_Init_80149318 (80149318), ftCo_800DEA28's FTKIND_CLINK arm:
/// ftCo_800DEAE8 enters the right row, or the left one facing left when
/// its animation exists, then cmd_vars[1] = 0.
pub fn enter_taunt(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.allow_interrupt = false;
    let state = if f.physics.facing == -1.0 && assets.left_taunt_available {
        FamilyState::AppealSL
    } else {
        FamilyState::AppealSR
    };
    f.change_motion_state(state.action(), assets)?;
    f.commands.variables[1] = 0;
    Ok(())
}

/// ftClink_Init_MotionStateTable rows 342/343: ftCo_SM_AppealSR/SL with
/// ftCl_AppealS_Anim, ftCo_AppealS_IASA, ft_80084F3C and ft_80084280.
pub const fn rows<C: LinkFamily>() -> [MotionRow; 2] {
    [
        row::<C>(FamilyState::AppealSR, 239),
        row::<C>(FamilyState::AppealSL, 240),
    ]
}

const fn row<C: LinkFamily>(state: FamilyState, animation: i32) -> MotionRow {
    MotionRow {
        animation,
        ..crate::row(
            state,
            taunt_animation::<C>,
            callbacks::input::appeal,
            callbacks::physics::guard_on,
            callbacks::collision::ground_wait,
        )
    }
}

/// ftCl_AppealS_Anim (80149354): on cmd_vars[1] = 1 with no milk out, the
/// milk at the left thumb (it_802C8B28), with ftLk_800EAF58 as the damage
/// and death2 callbacks; on 2 it is put away, as at the animation's end,
/// which returns to Wait (ft_8008A2BC).
fn taunt_animation<C: LinkFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let milk_out = f.character.get::<C>().specials_ref().milk_out;
    if f.commands.variables[1] == TAKE_OUT && !milk_out {
        take_out::<C>(f, p.assets);
    } else if f.commands.variables[1] == PUT_AWAY {
        put_away::<C>(f);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        put_away::<C>(f);
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// it_802C8B28 (802C8B28): the milk on the stage plane at Young Link's
/// position (Item_InitSpawnOnPlaneNoInitialCollision), taken into the left
/// thumb (Item_8026AB54).
fn take_out<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let thumb = assets.parts.joint(FtPart::LThumbNb).expect("Young Link thumb part");
    let c = &mut f.core;
    let spawn = SpawnItem::held(ItemKind::CLinkMilk, c.player.id, c.physics.position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: thumb,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    let specials = f.character.get_mut::<C>().specials();
    specials.milk_out = true;
    specials.removal_armed = true;
}

/// ftCl_Init_80149268 (80149268) -> checkFighter2244: the milk goes
/// (it_802C8C34, which forgets its owner first) and u.lk.x18 clears.
pub(crate) fn put_away<C: LinkFamily>(f: &mut Fighter) {
    if !std::mem::take(&mut f.character.get_mut::<C>().specials().milk_out) {
        return;
    }
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::CLinkMilk,
        control: ItemControl::Remove,
    });
}
