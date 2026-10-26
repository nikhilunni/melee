//! Escaping the stage's dangers and the screen's edge: ftCo_800A229C and
//! ftCo_800AE7AC, the head of every Versus CPU's decision.
use crate::{select, targets, world::Scene};
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;
use melee_ft::fighter::Fighter;
use melee_types::{GrKind, GroundOrAir};

/// What ftCo_800A229C returns besides its point: 1 for a stage's own
/// hazard (Hyrule Castle's tornado, Brinstar's acid), 2 for a stage moving
/// away (Great Bay's turtle, Rainbow Cruise, Big Blue, Icicle Mountain),
/// -1 outside the camera bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Escape {
    pub kind: i32,
    /// Where to escape toward (the stage's point, or the fighter itself).
    pub point: Vec3,
}

/// The escape kind of a fighter outside the camera bounds.
const OFF_CAMERA: i32 = -1;
/// A stage's own hazard: the attack modes stay.
const HAZARD: i32 = 1;
/// The stage leaving: the attack modes drop.
const STAGE_LEAVING: i32 = 2;

/// @322 (-5.0, double): Great Bay's escape starts right of this x.
const GREAT_BAY_ESCAPE_X: f64 = -5.0;
/// @325 (0.4, double): Icicle Mountain's margin per unit of scroll speed,
/// and its base.
const ICEMT_MARGIN: f64 = 0.4;

/// ftCo_800A229C (0x800A229C): whether the CPU must escape, and how.
pub fn escape_needed(fp: &Fighter, scene: &Scene) -> Option<Escape> {
    let stage = scene.stage();
    let position = fp.core.physics.position;
    let leave = Some(Escape {
        kind: STAGE_LEAVING,
        point: position,
    });
    // grGreatBay_801F66A4, then x > -5.0 in double (800A22DC).
    if scene.turtle_away && f64::from(position.x) > GREAT_BAY_ESCAPE_X {
        return leave;
    }
    if stage == GrKind::Castle {
        unimplemented!("ftCo_800A229C: grCastle_801CDF54 (Princess Peach's Castle)");
    }
    if let Some(acid) = scene.acid {
        // Brinstar: the acid's surface above the bottom blast zone; head
        // away from the bottom's middle (800A2348..800A2384).
        let bottom = scene.arena.bottom;
        if acid.surface > bottom {
            return Some(Escape {
                kind: HAZARD,
                point: Vec3::new(0.5 * (scene.arena.left + scene.arena.right), bottom, 0.0),
            });
        }
    }
    match stage {
        GrKind::RCruise | GrKind::BigBlue => {
            unimplemented!("ftCo_800A229C: {stage:?}'s dangers");
        }
        GrKind::Icemt if icicle_mountain_scrolled_away(position.y, scene) => return leave,
        _ => {}
    }
    // block_43: Camera_8003118C(&cur_pos, 0.0f).
    if !scene.camera.contains(position) {
        return Some(Escape {
            kind: OFF_CAMERA,
            point: position,
        });
    }
    None
}

/// ftCo_800A229C's Icicle Mountain arm (800A2604..800A26B4): the fighter
/// is within a margin of the blast zone the stage scrolls toward, the
/// margin `0.4 * |speed| + 0.4` (double fmadd, rounded) of the blast
/// zones' height.
fn icicle_mountain_scrolled_away(y: f32, scene: &Scene) -> bool {
    let arena = scene.arena;
    let height = arena.top - arena.bottom;
    let speed = crate::awareness::stage_drift_of(scene).y;
    let magnitude = if speed < 0.0 { -speed } else { speed };
    let margin = gekko_math::fma::fmadd(ICEMT_MARGIN, f64::from(magnitude), ICEMT_MARGIN) as f32;
    if f64::from(speed) < 0.0 {
        // fmadds height, margin, bottom.
        y < fmadds(height, margin, arena.bottom)
    } else {
        // fnmsubs height, margin, top: -(height * margin - top).
        y > gekko_math::fma::fnmsubs(height, margin, arena.top)
    }
}

/// ftCo_800AE7AC (0x800AE7AC): escape. A stage hazard (kind 1) keeps the
/// attack, recovery and defence modes (xF9_b2, b4, b6); a moving stage or
/// the screen's edge drops them. Off camera, a grounded CPU heads for the
/// stage's centre (ftCo_800A6FC4) or its target.
pub fn escape(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng, escape: Escape) {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b0 = true;
    cpu.xf9_b3 = false;
    cpu.xf9_b5 = false;
    cpu.xf9_b7 = false;
    cpu.xf9_b1 = false;
    let keep = escape.kind <= 1;
    cpu.xf9_b2 = keep;
    cpu.xf9_b4 = keep;
    cpu.xf9_b6 = keep;
    fp.core.cpu.target = targets::nearest_opponent(fp, scene, rng);
    targets::update_common_item(fp, scene);
    targets::update_special_item(fp, scene);
    if crate::should_act(&mut fp.core.cpu) {
        if escape.kind >= 0 {
            escape_from(fp, scene, rng, escape.point);
        } else if fp.core.physics.ground_or_air != GroundOrAir::Air {
            head_for_centre(fp, scene, rng);
        }
    }
    // ftCo_800AE7AC's frame under the think's.
    scene
        .stale
        .caller_frame
        .set(
            scene
                .stale
                .think_frame
                .get()
                .map(|frame| frame - crate::world::ESCAPE_FRAME_SIZE),
        );
    select::choose_behavior(fp, scene, rng);
}

/// ftCo_800AE7AC's grounded arm off camera: the island point nearest the
/// CPU toward the blast zones' centre (ftCo_800A6FC4), with a radius of 5
/// (ftCo_800A1F3C), else its target (ftCo_800A75DC).
fn head_for_centre(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    let arena = scene.arena;
    let position = fp.core.physics.position;
    // 0.5f * (right + left), 0.5f * (top + bottom): fadds, fmuls.
    let centre_x = 0.5 * (arena.right + arena.left);
    let centre_y = 0.5 * (arena.top + arena.bottom);
    let toward = Vec3::new(centre_x - position.x, centre_y - position.y, 0.0);
    if let Some(point) = island_point_toward(fp, scene, rng, toward) {
        crate::route::set_destination(fp, scene, point.x, point.y, 5.0);
    } else if let Some(target) = fp.core.cpu.target {
        // ftCo_800A75DC from this frame, not ftCo_800B24B8's: the stack
        // words it can read uninitialized are not the think's here.
        let think_frame = scene.stale.think_frame.replace(None);
        crate::route::toward_fighter(fp, scene, target);
        scene.stale.think_frame.set(think_frame);
    }
}

/// ftCo_800A6FC4 (0x800A6FC4): of the floor points over each other
/// island's middle and 5 inside each of its ends (probes 10 above to 10
/// below), the nearest within 45 degrees of `toward` and inside the blast
/// zones. Each island draws HSD_Randf first (800A7088), its result unused.
fn island_point_toward(
    fp: &Fighter,
    scene: &mut Scene,
    rng: &mut HsdRng,
    toward: Vec3,
) -> Option<Vec3> {
    /// @333 (0.7853982f): within this angle of `toward`.
    const WITHIN: f32 = core::f32::consts::FRAC_PI_4;
    let own = scene
        .map
        .island_of_line(fp.core.collision.data.floor.index);
    let toward = melee_lb::vector::normalize(toward);
    let position = fp.core.physics.position;
    let half_size = fp.core.cpu.half_size;
    let mut best: f32 = -1.0;
    let mut found = None;
    let mut next = scene.map.floor_islands().next();
    while let Some(node) = next {
        let island = *scene.map.island(node);
        next = island.next;
        if Some(node) == own {
            continue;
        }
        let (left, right) = (island.left, island.right);
        rng.randf();
        // 800A70A0..800A70E4: fadds, fmuls by 0.5f; y + 10.0 and y - 10.0
        // in double, rounded.
        let middle_x = 0.5 * (left.x + right.x);
        let middle_y = 0.5 * (left.y + right.y);
        let probes = [
            (middle_x, middle_y),
            // 800A725C: 5.0 + x in double; 800A73F0: x - 5.0.
            ((5.0 + f64::from(left.x)) as f32, left.y),
            ((f64::from(right.x) - 5.0) as f32, right.y),
        ];
        for (index, (x, y)) in probes.into_iter().enumerate() {
            let above = (10.0 + f64::from(y)) as f32;
            let below = (f64::from(y) - 10.0) as f32;
            let Some(hit) = scene.check_usable_floor(x, above, x, below) else {
                // A probe that finds no floor ends this island (800A7158,
                // 800A7304, 800A7498).
                break;
            };
            if index == 0 && scene.stage() == GrKind::RCruise {
                unimplemented!("ftCo_800A6FC4: grRCruise_80201988's floors");
            }
            let (fx, fy) = (hit.pos.x, hit.pos.y);
            let direction =
                melee_lb::vector::normalize(Vec3::new(fx - position.x, fy - position.y, 0.0));
            if melee_lb::vector::angle(toward, direction) > WITHIN {
                break;
            }
            let dx = fx - position.x;
            let dy = fy - position.y;
            // fmuls, fmuls, fadds.
            let distance = dx * dx + dy * dy;
            if scene.outside(fx, fy, half_size) {
                continue;
            }
            // The middle's point takes any first (best < 0.0 in double);
            // the ends only a nearer one (fcmpo, bge).
            let first = index == 0 && f64::from(best) < 0.0;
            if first || distance < best {
                best = distance;
                found = Some(Vec3::new(fx, fy, 0.0));
            }
        }
    }
    if f64::from(best) < 0.0 {
        None
    } else {
        found
    }
}

/// ftCo_800A8210 (0x800A8210): a grounded CPU heads away from a stage's
/// danger (within 5 of the point, through ftCo_800A1F3C's inline copy).
/// Airborne it keeps its course.
fn escape_from(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng, danger: Vec3) {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return;
    }
    let point = match scene.stage() {
        // The stage scrolls up (drift y < 0.0 in double): a platform
        // above; else an island below.
        GrKind::Icemt => {
            if f64::from(crate::awareness::stage_drift_of(scene).y) < 0.0 {
                platform_above(fp, scene)
            } else {
                island_below(fp, scene, rng)
            }
        }
        GrKind::RCruise | GrKind::BigBlue => {
            unimplemented!("ftCo_800A8210: {:?}'s drift", scene.stage())
        }
        _ => farthest_island_end(fp, scene, danger),
    };
    if let Some(point) = point {
        crate::route::set_destination(fp, scene, point.x, point.y, 5.0);
    }
}

/// ftCo_800A6700 (0x800A6700): of the points 5 inside each end of every
/// safe island (ftCo_800A2718) with a usable floor 5 above to 5 below,
/// inside the blast zones, the farthest from `from`.
fn farthest_island_end(fp: &Fighter, scene: &mut Scene, from: Vec3) -> Option<Vec3> {
    /// @296 (5.0, double): the inset from each end and the probe's reach.
    const INSET: f64 = 5.0;
    let half_size = fp.core.cpu.half_size;
    // @315: -1.0f.
    let mut best: f32 = -1.0;
    let mut found = None;
    let mut next = scene.map.floor_islands().next();
    while let Some(node) = next {
        let island = *scene.map.island(node);
        next = island.next;
        if crate::route::island_unsafe(scene, Some(node)) {
            continue;
        }
        // 800A67A0 / 800A691C: x + 5.0 and x - 5.0 in double, rounded;
        // the probe from y + 5.0 to y - 5.0 likewise.
        let ends = [
            ((f64::from(island.left.x) + INSET) as f32, island.left),
            ((f64::from(island.right.x) - INSET) as f32, island.right),
        ];
        for (x, end) in ends {
            let above = (INSET + f64::from(end.y)) as f32;
            let below = (f64::from(end.y) - INSET) as f32;
            if scene.check_usable_floor(x, above, x, below).is_none() {
                continue;
            }
            if scene.outside(x, end.y, half_size) {
                continue;
            }
            let dy = end.y - from.y;
            let dx = x - from.x;
            // 800A68AC / 800A6A28: fmuls, fmadds.
            let distance = fmadds(dx, dx, dy * dy);
            if distance > best {
                best = distance;
                found = Some(Vec3::new(x, end.y, end.z));
            }
        }
    }
    // 800A6A5C: best >= 0.0 in double.
    if f64::from(best) >= 0.0 {
        found
    } else {
        None
    }
}

/// The floor under an island's middle, probed from 5 above to 20 below
/// (fadds 0.5 * (right + left), fmuls; 5.0f + y, y - 20.0f in single).
fn island_middle_floor(scene: &mut Scene, island: &melee_mp::island::Island) -> Option<melee_mp::LineHit> {
    let x = 0.5 * (island.right.x + island.left.x);
    let y = 0.5 * (island.right.y + island.left.y);
    scene.check_usable_floor(x, 5.0 + y, x, y - 20.0)
}

/// The nearest of the candidates `pick` accepts (fmuls, fmuls, fadds; a
/// first one always), as ftCo_800A6A98 and ftCo_800A6D2C keep it.
fn nearer(best: &mut f32, found: &mut Option<Vec3>, position: Vec3, x: f32, y: f32) {
    let dx = x - position.x;
    let dy = y - position.y;
    let distance = dx * dx + dy * dy;
    if f64::from(*best) < 0.0 || distance < *best {
        *best = distance;
        *found = Some(Vec3::new(x, y, 0.0));
    }
}

/// ftCo_800A6A98 (0x800A6A98): Icicle Mountain scrolling up: the nearest
/// drop-through platform over a safe island's middle at or above the top
/// of the CPU's hurtbox (cur_pos.y + x568), inside the blast zones.
fn platform_above(fp: &Fighter, scene: &mut Scene) -> Option<Vec3> {
    let position = fp.core.physics.position;
    let top = position.y + fp.core.cpu.hurtbox_extents[3];
    let half_size = fp.core.cpu.half_size;
    let mut best: f32 = -1.0;
    let mut found = None;
    let mut next = scene.map.floor_islands().next();
    while let Some(node) = next {
        let island = *scene.map.island(node);
        next = island.next;
        if crate::route::island_unsafe(scene, Some(node)) {
            continue;
        }
        let Some(hit) = island_middle_floor(scene, &island) else {
            continue;
        };
        // 800A6C0C: mpLineGetFlags & LINE_FLAG_PLATFORM.
        if hit.line_id == -1
            || scene.map.line_get_flags(hit.line_id) & melee_types::mp::line_flag::PLATFORM == 0
        {
            continue;
        }
        let (x, y) = (hit.pos.x, hit.pos.y);
        if y < top || scene.outside(x, y, half_size) {
            continue;
        }
        nearer(&mut best, &mut found, position, x, y);
    }
    // 800A6CE4: best >= 0.0 in double.
    if f64::from(best) >= 0.0 {
        found
    } else {
        None
    }
}

/// ftCo_800A6D2C (0x800A6D2C): Icicle Mountain scrolling down: the
/// nearest floor over another safe island's middle more than an aerial
/// jump (x558) below the CPU, inside the blast zones. Each such island
/// draws HSD_Randf first (800A6DE0), its result unused.
fn island_below(fp: &Fighter, scene: &mut Scene, rng: &mut HsdRng) -> Option<Vec3> {
    let position = fp.core.physics.position;
    let low = position.y - fp.core.cpu.jump_height;
    let half_size = fp.core.cpu.half_size;
    let own = scene
        .map
        .island_of_line(fp.core.collision.data.floor.index);
    let mut best: f32 = -1.0;
    let mut found = None;
    let mut next = scene.map.floor_islands().next();
    while let Some(node) = next {
        let island = *scene.map.island(node);
        next = island.next;
        if crate::route::island_unsafe(scene, Some(node)) || Some(node) == own {
            continue;
        }
        rng.randf();
        let Some(hit) = island_middle_floor(scene, &island) else {
            continue;
        };
        let (x, y) = (hit.pos.x, hit.pos.y);
        // 800A6EC0: fcmpo, cror eq, gt, eq: skip at or above.
        if y >= low || scene.outside(x, y, half_size) {
            continue;
        }
        nearer(&mut best, &mut found, position, x, y);
    }
    // 800A6F7C: best >= 0.0 in double.
    if f64::from(best) >= 0.0 {
        found
    } else {
        None
    }
}
