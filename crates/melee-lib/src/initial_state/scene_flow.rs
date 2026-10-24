//! Match flow at a savestate boundary: the match clock (lbl_8046B6A0), the
//! running HUD banner (ifStatus_803F9628) and the rain's last drop
//! (stage_info.x9C), read from saved MEM1.
use super::{float, word};
use crate::{
    banner::{Banner, BannerKind},
    match_clock::{CountdownTimer, MatchClock},
};
use anyhow::{ensure, Result};
use melee_gr::bomb_rain::BombRain;

const MATCH_DATA: u32 = 0x8046_B6A0;
const HUD_ENABLED: usize = 0x05;
const FRAME_COUNT: usize = 0x24;
/// StartMeleeRules (+0x24C8) x6, the Sudden Death rule.
const SUDDEN_DEATH_RULE: usize = 0x24C8 + 6;
const TIMER_SECONDS: usize = 0x28;
const TIMER_FRAMES: usize = 0x2C;
/// StartMeleeRules byte 0: timer_enabled (bit 1) and timer_counts_up (bit 0).
const RULE_FLAGS: usize = 0x24C8;
const TIMER_ENABLED: u8 = 0x02;
const TIMER_COUNTS_UP: u8 = 0x01;
const BANNERS: u32 = 0x803F_9628;
const BANNER_BYTES: u32 = 0x28;
const BANNER_COUNT: u32 = 8;
const STAGE_INFO: u32 = 0x8049_E6C8;
const LAST_DROP_FRAME: u32 = 0x9C;
/// HSD_GObj.hsd_obj, HSD_JObj.child/next/aobj, HSD_AObj.curr_frame.
const GOBJ_OBJECT: usize = 0x28;
const JOBJ_NEXT: usize = 0x08;
const JOBJ_CHILD: usize = 0x10;
const JOBJ_AOBJ: usize = 0x7C;
const AOBJ_FRAME: usize = 0x04;

pub(super) fn restore_clock(saved: &super::SavedPose) -> Result<(MatchClock, BombRain)> {
    let data = saved.bytes(MATCH_DATA, SUDDEN_DEATH_RULE + 1);
    let flags = data[RULE_FLAGS];
    ensure!(
        flags & TIMER_COUNTS_UP == 0,
        "a counting-up match timer is not ported"
    );
    let clock = MatchClock {
        hud_enabled: data[HUD_ENABLED] != 0,
        frame_count: word(data, FRAME_COUNT),
        sudden_death: data[SUDDEN_DEATH_RULE] != 0,
        timer: (flags & TIMER_ENABLED != 0).then(|| CountdownTimer {
            seconds: word(data, TIMER_SECONDS),
            frames: u16::from_be_bytes([data[TIMER_FRAMES], data[TIMER_FRAMES + 1]]),
        }),
    };
    let rain = BombRain {
        last_drop_frame: word(saved.bytes(STAGE_INFO + LAST_DROP_FRAME, 4), 0),
    };
    Ok((clock, rain))
}

/// StartMeleeRules (+0x24C8) xB, the item frequency, and x20, the mask.
const ITEM_FREQUENCY_RULE: u32 = 0x24C8 + 0xB;
const ITEM_MASK_RULE: u32 = 0x24C8 + 0x20;
/// it_804A0E30 (RandomItemSpawner): x0 the countdown, x4 the pick table
/// (its total, a halfword, at +0xC), x18 the mask.
const ITEM_SPAWNER: u32 = 0x804A_0E30;
const SPAWNER_TOTAL: u32 = 0xC;
const SPAWNER_MASK: u32 = 0x18;

pub(super) fn restore_item_rules(saved: &super::SavedPose) -> melee_it::random_spawn::ItemRules {
    let frequency = saved.bytes(MATCH_DATA + ITEM_FREQUENCY_RULE, 1)[0] as i8;
    let mask = saved.bytes(MATCH_DATA + ITEM_MASK_RULE, 8);
    let mask = u64::from(word(mask, 0)) << 32 | u64::from(word(mask, 4));
    melee_it::random_spawn::ItemRules { frequency, mask }
}

/// The match's item rules and, when it_8026D018 made one, the random item
/// spawner: its tables rebuilt from the rules, its countdown from memory.
pub(super) fn restore_item_spawner(
    saved: &super::SavedPose,
    setup: &crate::setup::Setup,
    assets: &crate::assets::Assets,
) -> Result<Option<melee_it::random_spawn::RandomItemSpawner>> {
    let rules = restore_item_rules(saved);
    ensure!(
        !setup.items.enabled() || setup.items == rules,
        "the scenario names items {:?}, the savestate runs {rules:?}",
        setup.items
    );
    let sudden_death = saved.bytes(MATCH_DATA + SUDDEN_DEATH_RULE as u32, 1)[0] != 0;
    super::require_item_witness(
        rules,
        sudden_death,
        assets.stage_desc.kind,
        setup.fighters.len(),
    );
    if sudden_death {
        return Ok(None);
    }
    let common = &assets.items.common.random_items;
    let spawner = saved.bytes(ITEM_SPAWNER, 0x20);
    let Some(restored) = melee_it::random_spawn::RandomItemSpawner::restore(
        rules,
        &assets.item_rates,
        super::item_weight_scale(setup.fighters.len()),
        &common.monster_weights,
        word(spawner, 0) as i32,
    ) else {
        return Ok(None);
    };
    let saved_total = u16::from_be_bytes([
        spawner[SPAWNER_TOTAL as usize],
        spawner[SPAWNER_TOTAL as usize + 1],
    ]);
    let saved_mask = u64::from(word(spawner, SPAWNER_MASK as usize)) << 32
        | u64::from(word(spawner, SPAWNER_MASK as usize + 4));
    ensure!(
        saved_total == restored.items.total() && saved_mask == restored.mask,
        "the saved item spawner (total {saved_total}, mask {saved_mask:#x}) is not the one \
         the rules build (total {}, mask {:#x})",
        restored.items.total(),
        restored.mask
    );
    Ok(Some(restored))
}

/// FighterMatchInfo[i].x8 (lbl_8046B6A0 + 0x38 + i * 0xE + 8): the shared
/// revival offset slot timers.
pub(super) fn restore_revival_offsets(
    saved: &super::SavedPose,
) -> melee_ft::fighter::life::RevivalOffsets {
    const MATCH_INFO: u32 = 0x38;
    const MATCH_INFO_STRIDE: u32 = 0xE;
    const OFFSET_TIMER: u32 = 8;
    melee_ft::fighter::life::RevivalOffsets {
        timers: std::array::from_fn(|slot| {
            saved.bytes(
                MATCH_DATA + MATCH_INFO + slot as u32 * MATCH_INFO_STRIDE + OFFSET_TIMER,
                1,
            )[0]
        }),
    }
}

/// The one banner whose GObj is live: its kind and GObj.
fn running_banner(saved: &super::SavedPose) -> Result<Option<(BannerKind, u32)>> {
    let mut running = None;
    for index in 0..BANNER_COUNT {
        let gobj = word(saved.bytes(BANNERS + index * BANNER_BYTES, 4), 0);
        if gobj == 0 {
            continue;
        }
        ensure!(running.is_none(), "two HUD banners running at the boundary");
        let kind = match index {
            1 => BannerKind::SuddenDeathCountdown,
            3 => BannerKind::Countdown,
            4 => BannerKind::Go,
            _ => anyhow::bail!("HUD banner {index} running at the boundary"),
        };
        running = Some((kind, gobj));
    }
    Ok(running)
}

/// The countdown a match-start boundary is about to create (the boundary
/// precedes the scene's first scheduler tick): element 1 under the Sudden
/// Death rule (gm_Scene_SuddenDeath_OnEnter), 3 for Versus. Like a cold start,
/// it takes its first step at creation.
pub(super) fn match_start_countdown(
    clock: &MatchClock,
    interface: &hsd_archive::Archive,
) -> Result<Banner> {
    let kind = if clock.sudden_death {
        BannerKind::SuddenDeathCountdown
    } else {
        BannerKind::Countdown
    };
    Banner::from_archive(interface, kind)
}

/// The banner running at a later boundary, resumed at its AObjs' shared frame.
pub(super) fn restore_banner(
    saved: &super::SavedPose,
    interface: &hsd_archive::Archive,
) -> Result<Option<Banner>> {
    let mut running = None;
    if let Some((kind, gobj)) = running_banner(saved)? {
        let index = kind as u32;
        let root = word(saved.bytes(gobj + GOBJ_OBJECT as u32, 4), 0);
        let (frame, flags) = animation_state(saved, root)
            .ok_or_else(|| anyhow::anyhow!("HUD banner {index} has no animation"))?;
        running = Some(Banner::resume(interface, kind, frame, flags)?);
    }
    Ok(running)
}

/// The first AObj's current frame and flags; ifStatus_802F6EA4 requested
/// every AObj at frame zero together, so they advance in step.
fn animation_state(saved: &super::SavedPose, jobj: u32) -> Option<(f32, u32)> {
    if jobj == 0 {
        return None;
    }
    let raw = saved.bytes(jobj, 0x80);
    let aobj = word(raw, JOBJ_AOBJ);
    if aobj != 0 {
        let raw = saved.bytes(aobj, 8);
        return Some((float(raw, AOBJ_FRAME), word(raw, 0)));
    }
    animation_state(saved, word(raw, JOBJ_CHILD))
        .or_else(|| animation_state(saved, word(raw, JOBJ_NEXT)))
}
