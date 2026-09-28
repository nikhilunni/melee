//! Where the CPU heads (CpuFighter.x54): ftCo_800A1F3C and the choices
//! that call it.
use crate::world::Scene;
use hsd_types::{Vec2, Vec3};
use melee_ft::fighter::Fighter;
use melee_types::{GrKind, GroundOrAir};

/// ftCo_800A1F3C (0x800A1F3C): head for (x, y), arriving within `radius`,
/// unless a stage route's waypoint is pending (x60); then the stage's
/// route table may divert (ftCo_800A1CC4).
pub fn set_destination(fp: &mut Fighter, scene: &mut Scene, x: f32, y: f32, radius: f32) {
    let cpu = &mut fp.core.cpu;
    if cpu.route_timer == 0 {
        cpu.destination = Vec2::new(x, y);
        cpu.destination_radius = radius;
        stage_route(fp, scene);
    }
}

/// ftCo_800A1CC4 (0x800A1CC4) with ftCo_803C6594[grkind]: only Great Bay
/// and the Temple carry route tables.
pub fn stage_route(_fp: &mut Fighter, scene: &mut Scene) {
    if matches!(scene.stage(), GrKind::GreatBay | GrKind::Shrine) {
        unimplemented!("ftCo_800A1CC4: ftCo_803C6594 stage routes");
    }
}

/// ftCo_800A2718 (0x800A2718): whether the island is unsafe: an armed
/// Bob-omb or shell sits on it, or the stage marks it (Yoshi's Story's
/// Shy Guy platforms, Brinstar's acid, Onett's cars).
pub fn island_unsafe(scene: &mut Scene, island: Option<usize>) -> bool {
    let Some(island) = island else {
        return false;
    };
    for item in scene.items {
        use melee_types::ItemKind as K;
        if !item.hitbox_active || !matches!(item.kind, K::BombHei | K::GShell | K::RShell) {
            continue;
        }
        if scene.map.island_of_line(item.floor_line) == Some(island) {
            return true;
        }
    }
    match scene.stage() {
        GrKind::Story => unimplemented!("ftCo_800A2718: mpIsland_8005AC8C (Yoshi's Story)"),
        GrKind::Zebes | GrKind::Onett => {
            unimplemented!("ftCo_800A2718: {:?}'s hazard islands", scene.stage())
        }
        _ => false,
    }
}

/// ftCo_800A2170 (0x800A2170): both fighters stand on one island.
pub fn same_island(scene: &Scene, a: &Fighter, b: &Fighter) -> bool {
    if a.core.physics.ground_or_air == GroundOrAir::Air
        || b.core.physics.ground_or_air == GroundOrAir::Air
    {
        return false;
    }
    let Some(island) = scene.map.island_of_line(a.core.collision.data.floor.index) else {
        return false;
    };
    scene.map.island_of_line(b.core.collision.data.floor.index) == Some(island)
}

/// ftCo_800A21FC (0x800A21FC): the destination lies on the fighter's
/// island (the floor 5 above it to 10 below).
pub fn destination_on_island(fp: &Fighter, scene: &mut Scene) -> bool {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return false;
    }
    let Some(island) = scene.map.island_of_line(fp.core.collision.data.floor.index) else {
        return false;
    };
    let destination = fp.core.cpu.destination;
    // 5.0 + y in double, rounded.
    let probe = Vec3::new(destination.x, (5.0 + f64::from(destination.y)) as f32, 0.0);
    scene.map.island_below(probe, -10.0) == Some(island)
}

/// ftCo_800A7AAC (0x800A7AAC): head for the partner's player (the fighter
/// Nana follows), or where it will land, avoiding unsafe islands and
/// stopping short of an island's ends.
pub fn toward_partner(fp: &mut Fighter, scene: &mut Scene) {
    let Some((_, partner)) = scene.partner_of(fp) else {
        return;
    };
    let partner_position = partner.core.physics.position;
    let partner_ground = partner.core.physics.ground_or_air;
    let partner_floor = partner.core.collision.data.floor.index;
    let partner_extent = partner.core.cpu.hurtbox_extents[2];
    // 800A7C38: fadds.
    let radius = fp.core.cpu.x56c + partner_extent;
    if partner_ground == GroundOrAir::Air {
        let below = partner_position.y - 1000.0;
        let above = 10.0 + partner_position.y;
        let Some(hit) =
            scene.check_usable_floor(partner_position.x, above, partner_position.x, below)
        else {
            return;
        };
        let island = scene.map.island_of_line(hit.line_id);
        if island_unsafe(scene, island) {
            return;
        }
        set_destination(fp, scene, hit.pos.x, hit.pos.y, radius);
        let Some(island) = island else {
            return;
        };
        let island = *scene.map.island(island);
        stop_short_of_ends(fp, scene, island.left, island.right, radius);
    } else {
        let island = scene.map.island_of_line(partner_floor);
        if island_unsafe(scene, island) {
            return;
        }
        // 800A7DC0..800A7DDC: y -/+ 2.0 in double, rounded. The floor it
        // finds only decides which of two equal calls runs.
        let below = (f64::from(partner_position.y) - 2.0) as f32;
        let above = (2.0 + f64::from(partner_position.y)) as f32;
        let _ = scene.check_usable_floor(partner_position.x, above, partner_position.x, below);
        set_destination(fp, scene, partner_position.x, partner_position.y, radius);
        let partner = scene.partner_of(fp).expect("the partner").1;
        if same_island(scene, fp, partner) {
            return;
        }
        // Not while CliffCatch (0xFC) or CliffWait (0xFD).
        if matches!(fp.core.motion_state.action.0, 0xFC | 0xFD) {
            return;
        }
        let Some(island) = scene.map.island_of_line(partner_floor) else {
            return;
        };
        let island = *scene.map.island(island);
        let cpu = &fp.core.cpu;
        let position = fp.core.physics.position;
        if f64::from(cpu.destination.y - position.y) > 0.0 {
            if f64::from(cpu.destination.x - position.x) > 0.0 {
                if position.x < island.left.x {
                    let x = (5.0 + f64::from(island.left.x)) as f32;
                    set_destination(fp, scene, x, island.left.y, radius);
                }
            } else if position.x > island.right.x {
                let x = (f64::from(island.right.x) - 5.0) as f32;
                set_destination(fp, scene, x, island.right.y, radius);
            }
        }
    }
}

/// ftCo_800A7AAC's airborne tail: a destination within 5 of the island's
/// right end moves 5 inside it; else within 5 of the left end, likewise.
fn stop_short_of_ends(fp: &mut Fighter, scene: &mut Scene, left: Vec3, right: Vec3, radius: f32) {
    let destination = fp.core.cpu.destination.x;
    let mut d = right.x - destination;
    if d < 0.0 {
        d = -d;
    }
    if f64::from(d) < 5.0 {
        let x = (f64::from(right.x) - 5.0) as f32;
        set_destination(fp, scene, x, right.y, radius);
    } else {
        let mut d = left.x - destination;
        if d < 0.0 {
            d = -d;
        }
        if f64::from(d) < 5.0 {
            let x = (5.0 + f64::from(left.x)) as f32;
            set_destination(fp, scene, x, left.y, radius);
        }
    }
}
