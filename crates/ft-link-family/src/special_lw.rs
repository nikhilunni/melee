//! Bomb, ftlinkspeciallw.c (800EB6E8..800EBA4C).
//!
//! With a bomb already in hand the down special throws it forward as a
//! smash throw (ftCo_800957F4); with any other item nothing happens.
//! Otherwise Link reaches back and, on the script's throw flag, takes a
//! new bomb into his hand (it_8029DD58). The pull keeps its frame between
//! the grounded and aerial rows.
use crate::{common, row, Accessory, FamilyState, LinkFamily};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

fn bomb_kind<C: LinkFamily>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(f.character.get::<C>().attributes().bomb_item as i32).expect("bomb kind")
}

/// ftLk_SpecialLw_Enter (800EB6E8) / ftLk_SpecialAirLw_Enter (800EB714).
pub fn enter<C: LinkFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    // updateBomb: a bomb in hand is thrown (LightThrowF4 / AirF4); any
    // held item ends the entry.
    if let Some(held) = f.held_item {
        if matches!(held.kind, ItemKind::LinkBomb | ItemKind::CLinkBomb) {
            let throw = if air {
                CommonMotionState::LightThrowAirF4
            } else {
                CommonMotionState::LightThrowF4
            };
            f.enter_item_throw(throw, assets).expect("bomb throw assets");
        }
        return;
    }
    let state = if air {
        FamilyState::SpecialAirLw
    } else {
        FamilyState::SpecialLw
    };
    f.commands.clear_throw_flags();
    common::change(f, state.action(), 0, 0.0, assets).expect("Bomb assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    crate::arm_accessory::<C>(f, Accessory::PullBomb);
}

/// spawnBomb (800EB7C8), accessory4 of both rows every frame: on the
/// script's throw flag a bomb at Link's centre, swept from the left thumb
/// (it_8029DD58), taken into the hand at ftData x8 +0x10 with the catch
/// flag (ftpickupitem_80094818(gobj, 1)).
pub fn pull<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    // ftCheckThrowB0.
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let thumb = assets.parts.joint(FtPart::LThumbNb).expect("Link thumb part");
    let c = &mut f.core;
    let hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(thumb),
        hsd_types::Vec3::ZERO,
    );
    let center = c.item_holder(thumb, assets).center;
    let kind = bomb_kind::<C>(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::attached(kind, c.player.id, center, c.physics.facing);
    spawn.previous_position = hand;
    // ftParts_80074A4C(gobj, 2, 1) goes with the item in hand (see
    // hylian::raise).
    let part = c.bones.model.animation_translation;
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: true,
        catch_item: true,
        scale_by_owner: false,
    });
}

/// doAnim: Wait (ft_8008A2BC) or Fall (ftCo_Fall_Enter) at the end.
fn finish(f: &mut Fighter, p: AnimationPhase<'_>, air: bool) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    finish(f, p, false)
}
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    finish(f, p, true)
}

/// ftLk_SpecialLw_Phys -> ft_80084F3C.
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ftLk_SpecialAirLw_Phys -> ft_80084EEC.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftLk_SpecialLw_Coll (800EB9E8): ft_80082708; off the floor the aerial
/// row at the same frame, which keeps pulling.
fn collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Bomb collision assets");
        common::ground_to_air(f, FamilyState::SpecialAirLw.action(), assets)?;
        crate::arm_accessory::<C>(f, Accessory::PullBomb);
    }
    Ok(())
}

/// ftLk_SpecialAirLw_Coll (800EBA1C): ft_80081D0C; landing, the grounded
/// row at the same frame.
fn air_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Bomb landing assets");
        common::air_to_ground(f, FamilyState::SpecialLw.action(), assets)?;
        crate::arm_accessory::<C>(f, Accessory::PullBomb);
    }
    Ok(())
}

/// The two rows (ftLk_MS_SpecialLw / SpecialAirLw); no IASA.
pub const fn rows<C: LinkFamily>() -> [MotionRow; 2] {
    [
        row(
            FamilyState::SpecialLw,
            anim,
            common::no_input,
            physics,
            collision::<C>,
        ),
        row(
            FamilyState::SpecialAirLw,
            air_anim,
            common::no_input,
            air_physics,
            air_collision::<C>,
        ),
    ]
}
