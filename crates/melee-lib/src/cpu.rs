//! The scene side of the CPU proc: the view melee-cpu thinks with.
use crate::{frame::MAX_FIGHTERS, initial_state::InitialState};
use melee_ft::fighter::Fighter;

/// Fighter_8006ABA0 -> ftCo_800B3900 for fighter `index`, which the CPU
/// drives (see `Fighter::cpu_driven`).
pub(crate) fn think(state: &mut InitialState, index: usize) {
    // HSD_GObj_Entities->items in list order, as the CPU's item scans walk it.
    let mut items = [melee_cpu::ItemView::NONE; melee_it::ITEM_CAPACITY];
    let mut item_count = 0;
    let thinker = state.fighters[index].spawn_number;
    for item in state.items.iter().filter(|item| !item.destroyed) {
        let mut hitboxes = [None; 4];
        for (view, (hit, flags)) in hitboxes
            .iter_mut()
            .zip(item.hitboxes.iter().zip(&item.hit_flags))
        {
            *view = hit.as_ref().map(|hit| melee_cpu::ItemHitView {
                placed: hit.phase != melee_coll::hitbox::CapsulePhase::Enabled,
                element: hit.descriptor.element,
                position: hit.position,
                previous_position: hit.previous_position,
                radius: hit.descriptor.radius,
                absorbable: flags.absorbable,
                struck_thinker: hit.victims.contains(&thinker),
            });
        }
        items[item_count] = melee_cpu::ItemView {
            id: item.id,
            kind: item.kind,
            position: item.position,
            // Item_IsGrabbable: xDC8 x15 and a picked_up callback, which
            // every common item kind the CPU considers has.
            grabbable: item.grabbable,
            hitbox_active: item.hitboxes.iter().any(Option::is_some),
            floor_line: item.collision.as_ref().map_or(-1, |c| c.floor.index),
            owner: item.owner,
            hitboxes,
            motion: item.motion,
            life_timer: item.life_timer,
            bomb_fuse: (<crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(item.kind).fuse)(item),
        };
        item_count += 1;
    }
    // gm_8016C75C: the player's KO total, the KOs it scored on other
    // players (kos_by_player, which Player_UpdateKOsBySlot counts at a
    // stock loss, ftCo_800D34E0).
    let mut teams = [None; 4];
    for f in &state.fighters {
        teams[usize::from(f.player.id)] = f.player.team;
    }
    let player_kills = state.ko_counts.total(state.fighters[index].player.id, &teams);
    // gm_8016C6C0 (ftCo_800A53DC's ranking): the standings as they stand.
    let standings = crate::standings::current_ranks(state);
    let count = state.fighters.len();
    let deadzone = state.assets.fighters[index]
        .input
        .thresholds
        .horizontal_stick_deadzone;
    let (before, rest) = state.fighters.split_at_mut(index);
    let (own, after) = rest.split_first_mut().expect("the thinking fighter");
    let mut others: [Option<&Fighter>; MAX_FIGHTERS] = [None; MAX_FIGHTERS];
    for (slot, fighter) in others.iter_mut().zip(before.iter()) {
        *slot = Some(&fighter.0);
    }
    for (slot, fighter) in others[index + 1..].iter_mut().zip(after.iter()) {
        *slot = Some(&fighter.0);
    }
    let mut scene = melee_cpu::Scene {
        fighters: &others[..count],
        own: index,
        map: &mut state.map,
        arena: &state.stage_limits.arena,
        camera: &state.stage_limits.camera,
        items: &items[..item_count],
        data: &state.assets.cpu,
        player_kills,
        standings,
        horizontal_deadzone: deadzone,
        barrel_aim: match &state.stage {
            crate::scene_stage::SceneStage::Kongo(stage) => stage.barrel_angle,
            crate::scene_stage::SceneStage::OldKongo(stage) => stage.cannon_aim,
            _ => 0.0,
        },
        // grIceMt_801FA728 (Versus): (0, -map 9's x10, 0), the scroll's
        // speed as the controller last left it.
        stage_drift: match &state.stage {
            crate::scene_stage::SceneStage::IceMt(stage) => {
                Some(hsd_types::Vec3::new(0.0, -stage.controller.scroll.speed, 0.0))
            }
            crate::scene_stage::SceneStage::RCruise(stage) => Some(stage.drift),
            _ => None,
        },
        turtle_away: match &state.stage {
            crate::scene_stage::SceneStage::GreatBay(stage) => {
                stage.turtle.phase != melee_gr::greatbay::turtle::Phase::Surface
            }
            _ => false,
        },
        acid: match &state.stage {
            crate::scene_stage::SceneStage::Zebes(stage) => Some(melee_cpu::Acid {
                surface: stage.floor,
                motion: stage.floor_motion(),
            }),
            _ => None,
        },
        stale: Default::default(),
    };
    melee_cpu::think(&mut own.0, &mut scene, &mut state.rng);
    // ftCo_800B0AF4's last act: the partner joins the Blizzard
    // (ftPp_SpecialLw_Enter, the grounded entry whatever the partner's
    // footing).
    if std::mem::take(&mut own.0.cpu.joins_blizzard) {
        let fighter: &mut Fighter = &mut own.0;
        (fighter.character.table().enter_special)(
            fighter,
            melee_ft::fighter::SpecialSlot::Down,
            false,
            &state.assets.fighters[index],
        );
    }
}
