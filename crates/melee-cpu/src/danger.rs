//! Escaping the stage's dangers and the screen's edge: ftCo_800A229C and
//! ftCo_800AE7AC, the head of every Versus CPU's decision.
use crate::{select, targets, world::Scene};
use gekko_math::HsdRng;
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

/// ftCo_800A229C (0x800A229C): whether the CPU must escape, and how.
pub fn escape_needed(fp: &Fighter, scene: &Scene) -> Option<Escape> {
    let stage = scene.stage();
    if matches!(
        stage,
        GrKind::GreatBay | GrKind::Castle | GrKind::Zebes | GrKind::RCruise | GrKind::BigBlue
            | GrKind::Icemt
    ) {
        unimplemented!("ftCo_800A229C: {stage:?}'s dangers");
    }
    // block_43: Camera_8003118C(&cur_pos, 0.0f).
    let position = fp.core.physics.position;
    if !scene.camera.contains(position) {
        return Some(Escape {
            kind: OFF_CAMERA,
            point: position,
        });
    }
    None
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
            unimplemented!("ftCo_800A8210: escaping toward a stage's point");
        }
        if fp.core.physics.ground_or_air != GroundOrAir::Air {
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
