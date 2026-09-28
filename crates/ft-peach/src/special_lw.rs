//! Vegetable, ftpeachspeciallw.c (8011CE48..8011D424): Peach pulls a
//! turnip (rarely another item) out of the ground into her hand. With a
//! turnip already in hand the down special throws it instead; in the air
//! there is nothing to pull.
use crate::init::{Accessory, Peach};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, ItemKind};

/// ftPe_MS_SpecialLw (352) / ftPe_MS_SpecialAirLw (353).
pub const PULL: ActionId = ActionId(352);
pub const AIR_PULL: ActionId = ActionId(353);
/// coll_mf: SkipMatAnim | SkipColAnim | UpdateCmd | SkipItemVis | Unk19 |
/// SkipModelPartVis | SkipModelFlags | Unk27.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);
/// fp->parts[FtPart_109]: the pulled item appears at this joint.
const PULL_JOINT: usize = 109;
/// efSync_Spawn(1234, gobj, &fp->cur_pos) as the item comes up.
const PULL_EFFECT: u16 = 1234;

/// ftPe_SpecialLw_Enter (8011D11C) / ftPe_SpecialAirLw_Enter (8011D1C4).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    // throwVegIfHeld: any item in hand ends the entry; a turnip is thrown
    // forward as a smash throw (ftCo_800957F4).
    if let Some(held) = f.held_item {
        if held.kind == ItemKind::PeachTurnip {
            let throw = if air {
                CommonMotionState::LightThrowAirF4
            } else {
                CommonMotionState::LightThrowF4
            };
            f.enter_item_throw(throw, assets)
                .expect("turnip throw assets");
        }
        return;
    }
    if air {
        return;
    }
    f.commands.clear_throw_flags();
    f.change_motion_state(PULL, assets)
        .expect("Vegetable assets");
    f.step_animation(assets);
    arm_pull(f);
}

/// accessory4_cb = spawnVeg, until the next motion change.
fn arm_pull(f: &mut Fighter) {
    f.character.get_mut::<Peach>().accessory = Accessory::PullVegetable;
    f.core.arm_accessory4();
}

/// spawnVeg (8011D018), accessory4 of both rows: the script's throw flag
/// pulls the item up at joint 109.
pub fn pull(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    // ftCheckThrowB0.
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let c = &mut f.core;
    let position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        PULL_JOINT,
        hsd_types::Vec3::ZERO,
    );
    let kind = choose_item(f.character.get::<Peach>(), rng);
    if kind != ItemKind::PeachTurnip {
        unimplemented!("it_802BD4AC: Peach pulled a {kind:?}");
    }
    // it_802BD4AC -> it_802BD32C: the turnip's face, drawn before it is
    // attached; the item reads it back from the spawn.
    let face = choose_face(&f.character.get::<Peach>().attributes.turnip_faces, rng);
    // Item_InitSpawn at the joint; Item_8026AB54 at ftData x8 +0x10.
    let spawn = SpawnItem {
        spawn_argument: face,
        ..SpawnItem::attached(kind, f.player.id, position, f.physics.facing)
    };
    let part = f.core.bones.model.animation_translation;
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: true,
    });
    // setupVeg: the pull's effect, then death2_cb and take_dmg_cb.
    let position = f.physics.position;
    f.effects
        .push(melee_ef::request::EffectRequest::PositionalModel {
            id: PULL_EFFECT,
            position,
        });
    let items = &mut f.character.get_mut::<Peach>().items;
    items.vegetable = true;
    items.death2_armed = true;
    items.take_damage_armed = true;
}

/// getVeg (inlined in spawnVeg): a turnip unless HSD_Randi(+14) draws zero,
/// then pickVeg (8011CE48) weighs the rare items.
fn choose_item(peach: &Peach, rng: &mut gekko_math::HsdRng) -> ItemKind {
    let vegetable = &peach.attributes.vegetable;
    if rng.randi(vegetable.rare_item_odds) != 0 {
        return ItemKind::PeachTurnip;
    }
    let table = &vegetable.rare_items;
    let count = vegetable.rare_item_count as usize;
    let odds = table.iter().take(count).map(|c| c.weight).sum();
    let draw = rng.randi(odds);
    table[weighted_index(draw, count, |i| table.get(i).map_or(0, |c| c.weight))].kind
}

/// it_802BD32C (802BD32C): the turnip's face, weighted by its odds.
fn choose_face(faces: &crate::attributes::TurnipFaces, rng: &mut gekko_math::HsdRng) -> i32 {
    let odds = faces.weights[..faces.count].iter().sum();
    let draw = rng.randi(odds);
    weighted_index(draw, faces.count, |i| {
        faces.weights.get(i).copied().unwrap_or(0)
    }) as i32
}

/// pickVeg / it_802BD32C's walk: the first entry whose running total
/// passes `draw`. Retail reads one entry past the last while summing.
fn weighted_index(draw: i32, count: usize, weight: impl Fn(usize) -> i32) -> usize {
    let mut total = weight(0);
    for i in 0..count {
        if draw < total {
            return i;
        }
        total += weight(i + 1);
    }
    unreachable!("pickVeg: a draw past every weight")
}

/// ftPe_SpecialLw_8011CFA0 (8011CFA0), from ftPe_Init_OnDeath2: a turnip
/// still in hand during the pull is destroyed (it_802BD45C); the scene
/// releases the hand as the item goes (Item_8026A8EC).
pub fn put_away(f: &mut Fighter) {
    if !f.character.get::<Peach>().items.vegetable {
        return;
    }
    let Some(held) = f.held_item else {
        return;
    };
    if held.kind != ItemKind::PeachTurnip {
        return;
    }
    f.core
        .item_requests
        .push(ItemRequest::Destroy { item: held.item });
    f.character.get_mut::<Peach>().items.vegetable = false;
}

/// ftPe_SpecialLw_Anim (8011D2EC) / ftPe_SpecialAirLw_Anim (8011D340):
/// Wait or Fall at the end, and the pull lets go of the new item
/// (ftPe_SpecialLw_UnsetVeg).
fn finish(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    next: CommonMotionState,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(next.into(), p.assets)?;
        f.character.get_mut::<Peach>().items.vegetable = false;
    }
    Ok(None)
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    finish(f, p, CommonMotionState::Wait)
}
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    finish(f, p, CommonMotionState::Fall)
}

/// ftPe_SpecialLw_Phys -> ft_80084F3C.
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ftPe_SpecialAirLw_Phys -> ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftPe_SpecialLw_Coll (8011D3D4): ft_8008403C; off the floor the aerial
/// row (handleAirColl), which keeps pulling.
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    if map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) != WaitGroundResult::Supported
    {
        f.leave_ground();
        change_in_place(f, AIR_PULL, p.assets.expect("Vegetable collision assets"))?;
    }
    Ok(())
}

/// ftPe_SpecialAirLw_Coll (8011D3FC): ft_80082C74; landing, the grounded
/// row (handleColl).
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        f.land();
        change_in_place(f, PULL, p.assets.expect("Vegetable landing assets"))?;
    }
    Ok(())
}

/// ftCommon_GroundToAirStateChange / AirToGroundStateChange with coll_mf,
/// then accessory4_cb = spawnVeg again.
fn change_in_place(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    arm_pull(f);
    Ok(())
}
