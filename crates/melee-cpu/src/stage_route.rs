//! ftCo_800A1CC4 with ftCo_803C6594: the stage route tables. On an island
//! whose way to the destination is blocked, a waypoint replaces the
//! destination for 300 frames (x60), and x64 keeps the destination.
use crate::world::Scene;
use hsd_types::{Vec2, Vec3};
use melee_ft::fighter::Fighter;
use melee_types::{GrKind, GroundOrAir};

/// A waypoint; disabled when `enabled` is negative (the table's third
/// float of each triple, tested `>= 0.0`).
#[derive(Clone, Copy)]
struct Waypoint {
    x: f32,
    y: f32,
    enabled: f32,
}

/// One ftCo_803C6594_t (0x54 bytes).
struct RouteEntry {
    /// x0: the point whose island (10 below) the fighter must stand on.
    island_probe: [f32; 2],
    /// xC: when the destination lies above `up_angle`.
    up: Waypoint,
    /// x18: when the destination lies below `down_angle`.
    down: Waypoint,
    /// x24: otherwise, facing right.
    right: Waypoint,
    /// x30: otherwise, facing left.
    left: Waypoint,
    /// x3C, x40.
    up_angle: f32,
    down_angle: f32,
    /// x44, x48, x4C: when set, only strictly between these x.
    x_range: Option<(f32, f32)>,
}

const fn w(x: f32, y: f32, enabled: f32) -> Waypoint {
    Waypoint { x, y, enabled }
}

/// The table's pi/4 and pi/12 (0.7853982, 0.2617994).
const QUARTER: f32 = core::f32::consts::FRAC_PI_4;
const TWELFTH: f32 = 0.261_799_4;

/// ftCo_803C6594[Gr_Kind_Shrine] (ft_3C61.c list0_0..list0_4).
#[rustfmt::skip]
static TEMPLE: [RouteEntry; 5] = [
    RouteEntry { island_probe: [37.0, -143.5], up: w(5.5, -111.0, 0.0), down: w(5.5, -111.0, 0.0),
        right: w(0.0, 0.0, -1.0), left: w(5.5, -111.0, 0.0), up_angle: QUARTER, down_angle: -QUARTER, x_range: None },
    RouteEntry { island_probe: [30.0, -16.0], up: w(-3.5, 24.0, 0.0), down: w(-60.0, -86.0, 0.0),
        right: w(0.0, 0.0, -1.0), left: w(-3.5, 24.0, 0.0), up_angle: QUARTER, down_angle: -TWELFTH, x_range: None },
    RouteEntry { island_probe: [-60.0, -86.0], up: w(-131.0, -26.8, 0.0), down: w(0.0, 0.0, -1.0),
        right: w(0.0, 0.0, -1.0), left: w(0.0, 0.0, -1.0), up_angle: QUARTER, down_angle: -QUARTER,
        x_range: Some((-1000.0, -95.0)) },
    RouteEntry { island_probe: [-60.0, -86.0], up: w(-16.0, -46.0, 0.0), down: w(0.0, 0.0, -1.0),
        right: w(0.0, 0.0, -1.0), left: w(0.0, 0.0, -1.0), up_angle: TWELFTH, down_angle: -QUARTER,
        x_range: Some((-95.0, -25.0)) },
    RouteEntry { island_probe: [-60.0, -86.0], up: w(-60.0, -86.0, 0.0), down: w(0.0, 0.0, -1.0),
        right: w(0.0, 0.0, -1.0), left: w(-60.0, -86.0, 0.0), up_angle: QUARTER, down_angle: -QUARTER,
        x_range: Some((-25.0, 1000.0)) },
];

/// x60 = 300 when a waypoint is taken (800A1DF4: li r0, 0x12C).
const WAYPOINT_FRAMES: i32 = 300;
/// x38 = 5.0 for a waypoint (800A1E18: @297).
const WAYPOINT_RADIUS: f32 = 5.0;
/// mpIsland_8005AC14's probe depth (800A1D78: @321).
const PROBE_DEPTH: f32 = -10.0;

fn table(stage: GrKind) -> Option<&'static [RouteEntry]> {
    match stage {
        GrKind::Shrine => Some(&TEMPLE),
        GrKind::GreatBay => unimplemented!("ftCo_800A1CC4: Great Bay's route table (ft_3C61.c list1_0)"),
        _ => None,
    }
}

/// ftCo_800A1CC4 (0x800A1CC4).
pub fn apply(fp: &mut Fighter, scene: &mut Scene) {
    let Some(entries) = table(scene.stage()) else {
        return;
    };
    let cpu = &fp.core.cpu;
    if cpu.route_timer != 0
        || cpu.mode == 0
        || fp.core.physics.ground_or_air == GroundOrAir::Air
        || crate::route::destination_on_island(fp, scene)
    {
        return;
    }
    let Some(island) = scene
        .map
        .island_of_line(fp.core.collision.data.floor.index)
    else {
        return;
    };
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    // 800A1D3C..800A1D60: fsubs each, |dx| by fcmpo against 0 and fneg.
    let dx = destination.x - position.x;
    let dy = destination.y - position.y;
    let dx = if dx >= 0.0 { dx } else { -dx };
    let angle = melee_lb::trigf::stick_angle(dy, dx);
    for entry in entries {
        let probe = Vec3::new(entry.island_probe[0], entry.island_probe[1], 0.0);
        if scene.map.island_below(probe, PROBE_DEPTH) != Some(island) {
            continue;
        }
        if let Some((low, high)) = entry.x_range {
            if !(position.x > low && position.x < high) {
                continue;
            }
        }
        let waypoint = if angle > entry.up_angle {
            entry.up
        } else if angle < entry.down_angle {
            entry.down
        } else if fp.core.physics.facing > 0.0 {
            entry.right
        } else {
            entry.left
        };
        // 800A1DE4: fcmpo, cror eq, gt, eq: `>= 0.0`; a disabled
        // waypoint tries the next entry.
        if waypoint.enabled >= 0.0 {
            let cpu = &mut fp.core.cpu;
            cpu.route_timer = WAYPOINT_FRAMES;
            cpu.route_destination = cpu.destination;
            cpu.destination = Vec2::new(waypoint.x, waypoint.y);
            cpu.destination_radius = WAYPOINT_RADIUS;
            return;
        }
    }
}
