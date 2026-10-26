//! ftCo_800AD7FC (0x800AD7FC, ftCo_0A01.c:5926): behaviour 14, the held
//! item thrown at the target by its bearing.
use crate::{attack::facing_target, script, world::Scene};
use melee_ft::fighter::Fighter;
use melee_types::ItemKind;

/// The CPU mode that throws its item at an angle of its own (0x1D).
const AIMED_THROW_MODE: i32 = 0x1D;
/// 75 and 15 degrees, the double literals ftCo_800AD7FC compares the
/// target's bearing with.
const STEEP_75: f64 = 1.3089969288557768;
const LEVEL_15: f64 = 0.26179938577115536;

/// PlCo's stored throw scripts.
mod scripts {
    /// Up and down throws.
    pub const UP: usize = 0xD;
    pub const DOWN: usize = 0xE;
    /// Forward.
    pub const FORWARD: usize = 0xF;
    /// Turned round and thrown (a back throw).
    pub const BACK: usize = 0x28;
    /// Mode 0x1D's forward, up and down throws.
    pub const AIMED_FORWARD: usize = 0x2A;
    pub const AIMED_UP: usize = 0x2B;
    pub const AIMED_DOWN: usize = 0x2C;
}

/// ftCo_800AD7FC (0x800AD7FC). With nothing in hand the CPU goes back to
/// its previous behaviour.
pub fn throw_item(fp: &mut Fighter, scene: &Scene) {
    let held = match (&fp.core.held_item, &fp.core.article_in_hand) {
        (Some(held), _) => held.kind,
        (None, Some(article)) => article.kind,
        (None, None) => {
            script::return_to_previous(&mut fp.core.cpu);
            return;
        }
    };
    let target = fp.core.cpu.target;
    if target.is_some() && fp.core.cpu.mode == AIMED_THROW_MODE {
        if !facing_target(fp, scene) {
            crate::behave::turn_around(fp);
            return;
        }
        let bearing = bearing(fp, scene);
        let id = if bearing > STEEP_75 {
            scripts::AIMED_UP
        } else if bearing < -STEEP_75 {
            scripts::AIMED_DOWN
        } else {
            scripts::AIMED_FORWARD
        };
        script::stored(&mut fp.core.cpu, scene.data, id);
        fp.core.cpu.behavior = fp.core.cpu.home_behavior;
        return;
    }
    // A capsule, an egg or a Poké Ball goes down rather than back.
    let rolled = matches!(held, ItemKind::Capsule | ItemKind::Egg | ItemKind::MBall);
    let away = if rolled { scripts::DOWN } else { scripts::BACK };
    let id = if target.is_some() {
        // 5960: ftCo_800A1AB4's distance is computed and dropped.
        let bearing = bearing(fp, scene);
        if bearing > STEEP_75 {
            scripts::UP
        } else if bearing < -STEEP_75 {
            scripts::DOWN
        } else if facing_target(fp, scene) && bearing < LEVEL_15 && bearing > -LEVEL_15 {
            scripts::FORWARD
        } else {
            away
        }
    } else {
        away
    };
    script::stored(&mut fp.core.cpu, scene.data, id);
    fp.core.cpu.behavior = fp.core.cpu.home_behavior;
}

/// lb_8000D008 of the target from the CPU (dy, |dx|: two fsubs, fneg
/// below zero), compared in double.
fn bearing(fp: &Fighter, scene: &Scene) -> f64 {
    let target = scene.fighter(fp.core.cpu.target.expect("a target"));
    let (own, theirs) = (fp.core.physics.position, target.core.physics.position);
    let mut across = theirs.x - own.x;
    if across < 0.0 {
        across = -across;
    }
    f64::from(melee_lb::trigf::stick_angle(theirs.y - own.y, across))
}
