//! Who and what the CPU goes after: ftCo_800A4BEC and the item choices.
use crate::world::{distance, ItemView, Scene};
use gekko_math::HsdRng;
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, ItemKind};

/// ftCo_IsAlly (0x800A38D4): fighters of one player, or in a team battle
/// (gm_8016B168) of one team; a Versus free-for-all (gm_8016B14C) has no
/// other allies.
fn ally(a: &Fighter, b: &Fighter) -> bool {
    a.core.player.id == b.core.player.id || a.core.player.teammate_of(b.core.player.team)
}

/// ftCo_800A1C44 (0x800A1C44): out of play (x2219_b1), or the star or
/// hammer music playing (x2164/x2168, which only those items start), or
/// x221F_b3.
fn unavailable(f: &Fighter) -> bool {
    f.core.out_of_play() || f.core.status.disabled
}

/// inlineD1 (ftCo_0A01.c:2415): x221F_b3, a stamina KO (x2224_b2, never
/// in stock rules), the cloaking device flickering (ftCo_800A0F00: needs
/// the item) or a death motion (ftLib_8008732C).
fn not_targetable(f: &Fighter) -> bool {
    f.core.status.disabled || f.core.motion_state.action.0 <= S::DeadUpFallHitCameraIce as u16
}

/// What ftCo_800A4A40 reads of another fighter.
#[derive(Clone, Copy, Debug)]
pub struct FighterView {
    /// cur_pos.
    pub position: hsd_types::Vec3,
    /// Not inlineD1: in play and not in a death motion.
    pub targetable: bool,
}
impl FighterView {
    pub fn of(f: &Fighter) -> Self {
        Self {
            position: f.core.physics.position,
            targetable: !not_targetable(f),
        }
    }
}

/// ftCo_800A4A40 (0x800A4A40): the position of the nearest other fighter,
/// whoever's side it is on, that is inside the blast zones shrunk by the
/// asking fighter's CPU half size (inlineD0) and targetable (inlineD1). Of
/// two equally near the earlier in list order stays (800A4BAC: fcmpo, ble).
pub fn nearest_fighter(
    position: hsd_types::Vec3,
    half_size: [f32; 2],
    arena: &melee_ft::fighter::life::Arena,
    others: impl Iterator<Item = FighterView>,
) -> Option<hsd_types::Vec3> {
    let mut closest: Option<(hsd_types::Vec3, f32)> = None;
    for other in others {
        let p = other.position;
        if crate::world::outside(arena, p.x, p.y, half_size) || !other.targetable {
            continue;
        }
        let d = distance(position, p);
        match closest {
            Some((_, best)) if best.partial_cmp(&d) != Some(core::cmp::Ordering::Greater) => {}
            _ => closest = Some((p, d)),
        }
    }
    closest.map(|(p, _)| p)
}

/// ftCo_800A4BEC (0x800A4BEC): the nearest fighter the CPU can fight, or
/// the locked one while the lock (xF9_b0) holds. Choosing a new target
/// locks it for a random time.
pub fn nearest_opponent(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) -> Option<usize> {
    let position = fp.core.physics.position;
    let half_size = fp.core.cpu.half_size;
    let mut closest: Option<(usize, f32)> = None;
    for (index, other) in scene.others() {
        let p = other.core.physics.position;
        if scene.outside(p.x, p.y, half_size) {
            continue;
        }
        scene
            .stale
            .frame_lr_slot
            .set(Some(crate::world::return_address::AFTER_IS_ALLY));
        if ally(fp, other) {
            continue;
        }
        // 800A1C44 inlined, then x221F_b3 and x2224_b2 before
        // ftCo_800A0F00 (800A4D44).
        if unavailable(other) {
            continue;
        }
        scene
            .stale
            .frame_lr_slot
            .set(Some(crate::world::return_address::AFTER_CLOAK_TEST));
        if not_targetable(other) {
            continue;
        }
        if fp.core.cpu.target_locked {
            if Some(index) == fp.core.cpu.locked_target {
                return Some(index);
            }
            continue;
        }
        let d = distance(position, p);
        match closest {
            Some((_, best)) if best.partial_cmp(&d) != Some(core::cmp::Ordering::Greater) => {}
            _ => closest = Some((index, d)),
        }
    }
    let cpu = &mut fp.core.cpu;
    match closest {
        None => {
            cpu.target_locked = false;
            None
        }
        Some((index, _)) => {
            cpu.target_locked = true;
            cpu.target_lock_timer = crate::awareness::lock_duration(rng);
            cpu.locked_target = Some(index);
            Some(index)
        }
    }
}

/// ftCo_803C5A68 (0x803C5A68): how much the CPU wants each common item
/// (kinds below It_Kind_L_Gun_Ray), read from the DOL.
const ITEM_PRIORITY: [i32; 35] = [
    1, 0, 0, 1, 0, 0, 0, 1, 7, 6, 5, 4, 4, 3, 1, 1, 2, 1, 5, 2, 2, 2, 3, 3, 3, 4, 0, 0, 8, 4, 1, 4,
    4, 5, 5,
];

fn priority(kind: ItemKind) -> i32 {
    ITEM_PRIORITY[i32::from(kind) as usize]
}

/// ftCo_GetItemDistance (inlined): sqrtf of the fused square sum.
fn item_distance(fp: &Fighter, item: &ItemView) -> f32 {
    distance(fp.core.physics.position, item.position)
}

/// ftCo_800A5908 (0x800A5908): healing items.
fn healing(kind: ItemKind) -> bool {
    matches!(kind, ItemKind::Heart | ItemKind::Tomato | ItemKind::Foods)
}

/// ftCo_CpuUpdateCommonItemTarget (inlined in ftCo_800B0760 and others):
/// x4C, the common item to go for, unless holding a non-healing item (or
/// the hammer, x2168, which only its item sets).
pub fn update_common_item(fp: &mut Fighter, scene: &Scene) {
    let holding = match (&fp.core.held_item, &fp.core.article_in_hand) {
        (Some(item), _) => Some(healing(item.kind)),
        (None, Some(_)) => Some(false),
        (None, None) => None,
    };
    fp.core.cpu.item_target = match holding {
        Some(false) => None,
        _ => common_item(fp, scene, None),
    };
}

/// ftCo_800A5F4C (0x800A5F4C): the nearest grabbable common item of the
/// highest priority, of `kind` if given (It_Kind_L_Gun_Ray means any).
fn common_item(fp: &Fighter, scene: &Scene, kind: Option<ItemKind>) -> Option<u32> {
    let half_size = fp.core.cpu.half_size;
    let mut closest: Option<(ItemView, f32)> = None;
    for item in scene.items {
        if !item.grabbable {
            continue;
        }
        if kind.is_some_and(|kind| item.kind != kind) {
            continue;
        }
        if scene.outside(item.position.x, item.position.y, half_size) {
            continue;
        }
        if i32::from(item.kind) >= i32::from(ItemKind::LGunRay) {
            continue;
        }
        if priority(item.kind) < fp.core.cpu.x2c {
            continue;
        }
        let Some((best_item, best)) = closest else {
            closest = Some((*item, item_distance(fp, item)));
            continue;
        };
        if priority(item.kind) < priority(best_item.kind) {
            continue;
        }
        let d = item_distance(fp, item);
        // 800A6184: the inlined sqrtf of a positive square stores through
        // sp1C, ftCo_800B24B8's r1 - 0x5C.
        if d > 0.0 {
            scene.stale.frame_lr_slot.set(Some(d.to_bits()));
        }
        if best > d {
            closest = Some((*item, d));
        }
    }
    closest.map(|(item, _)| item.id)
}

/// ftCo_800A648C (0x800A648C) through ftCo_CpuUpdateSpecialItemTarget:
/// x50, the nearest stage enemy in bounds (Goombas to Octaroks, Koopas,
/// Polar Bears).
pub fn update_special_item(fp: &mut Fighter, scene: &Scene) {
    let half_size = fp.core.cpu.half_size;
    let mut closest: Option<(u32, f32)> = None;
    for item in scene.items {
        let kind = i32::from(item.kind);
        let enemy = (i32::from(ItemKind::Kuriboh)..i32::from(ItemKind::OctarockStone))
            .contains(&kind)
            || item.kind == ItemKind::Nokonoko
            || (kind - i32::from(ItemKind::Patapata)) as u32 <= 1
            || item.kind == ItemKind::Whitebea;
        if !enemy || scene.outside(item.position.x, item.position.y, half_size) {
            continue;
        }
        let d = item_distance(fp, item);
        match closest {
            Some((_, best)) if best.partial_cmp(&d) != Some(core::cmp::Ordering::Greater) => {}
            _ => closest = Some((item.id, d)),
        }
    }
    fp.core.cpu.x50 = closest.map_or(0, |(id, _)| id);
}

/// ftCo_800A5CE0 (0x800A5CE0): the nearest opponent powered by the Starman
/// (x221D_b6) or swinging the Hammer (x2168), whom the CPU flees; none for
/// an untouchable CPU. Only those items power a fighter so, and no item the
/// port runs is one.
pub fn star_carrier(_fp: &Fighter, _scene: &Scene) -> Option<usize> {
    None
}

/// ftCo_800A53DC (0x800A53DC): the opponent the CPU goes after: of those in
/// bounds, available and targetable, the best placed in the standings
/// (gm_8016C6C0), then the nearest. With xFA_b34 set it first looks among
/// the players a person controls (not ftCo_800A2040), then among all.
pub fn attack_target(fp: &Fighter, scene: &Scene) -> Option<usize> {
    let candidates = |humans_only: bool| {
        let mut best: Option<(usize, u8, f32)> = None;
        for (index, other) in scene.others() {
            let p = other.core.physics.position;
            if scene.outside(p.x, p.y, fp.core.cpu.half_size)
                || ally(fp, other)
                || unavailable(other)
                || not_targetable(other)
                || (humans_only
                    && other.input_source() == melee_ft::input::human::InputSource::Cpu)
            {
                continue;
            }
            // 800A55E4: ftCo_800A1AB4.
            let d = distance(fp.core.physics.position, p);
            let player = other.core.player.id;
            match best {
                None => best = Some((index, player, d)),
                // gm_8016C6C0 ranks a player's slot (ftLib_80086BE0): one
                // player's two fighters (Popo and Nana) rank alike, and the
                // strictly nearer wins.
                Some((_, best_player, best_d)) if best_player == player => {
                    if best_d > d {
                        best = Some((index, player, d));
                    }
                }
                Some(_) => unimplemented!(
                    "ftCo_800A53DC: two players' opponents ranked by gm_8016C6C0's standings"
                ),
            }
        }
        best.map(|(index, _, _)| index)
    };
    if fp.core.cpu.xfa_b34 != 0 {
        if let Some(index) = candidates(true) {
            return Some(index);
        }
    }
    candidates(false)
}

/// ftCo_800A4E8C (0x800A4E8C): the opponent nearest `point` in three
/// dimensions, of those in bounds, available and targetable. The square
/// sum fuses x and z onto y's square (800A504C/800A5050: fmadds), then
/// sqrtf__Ff.
pub fn nearest_opponent_to(fp: &Fighter, scene: &Scene, point: hsd_types::Vec3) -> Option<usize> {
    let half_size = fp.core.cpu.half_size;
    let mut closest: Option<(usize, f32)> = None;
    for (index, other) in scene.others() {
        let p = other.core.physics.position;
        if scene.outside(p.x, p.y, half_size)
            || ally(fp, other)
            || unavailable(other)
            || not_targetable(other)
        {
            continue;
        }
        let dy = p.y - point.y;
        let dx = p.x - point.x;
        let dz = p.z - point.z;
        let d = gekko_math::msl::sqrtf(gekko_math::fma::fmadds(
            dz,
            dz,
            gekko_math::fma::fmadds(dx, dx, dy * dy),
        ));
        match closest {
            Some((_, best)) if best.partial_cmp(&d) != Some(core::cmp::Ordering::Greater) => {}
            _ => closest = Some((index, d)),
        }
    }
    closest.map(|(index, _)| index)
}
