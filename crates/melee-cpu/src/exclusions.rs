//! ftCo_800B77E8: the attack scripts a kind leaves out of the close attack
//! choice, mostly its ranged specials when the target is out of sight or
//! there is no floor ahead to land on.
use crate::world::Scene;
use melee_ft::fighter::Fighter;
use melee_types::GroundOrAir;

/// PlCo attack scripts (ftCo_AttackEntry.cmd) the kinds exclude.
mod scripts {
    /// Excluded for CPU mode 7 or an untouchable CPU.
    pub const MODE_7: i32 = 0x34;
    /// The side and down specials the kinds drop.
    pub const SPECIAL: i32 = 0x26;
    /// Kirby's Inhale, and its other form.
    pub const INHALE: i32 = 0x11;
    pub const INHALE_OTHER: i32 = 0x12;
}

/// The CPU mode whose CPUs leave the MODE_7 script out.
const MODE_7: i32 = 7;
/// ftCo_CpuCanUseRangedAttack's floor probe: 5 above to 1000 below the
/// point `range` ahead (each a double sum, rounded).
const PROBE_ABOVE: f64 = 5.0;
const PROBE_BELOW: f64 = 1000.0;

/// ftCo_800B77E8 (0x800B77E8). The scripts go to xCC (at most 8).
pub fn exclude_for_kind(fp: &mut Fighter, scene: &mut Scene) {
    if fp.core.cpu.mode == MODE_7 || crate::attack::untouchable(fp) {
        fp.core.cpu.excluded_attacks.push(scripts::MODE_7);
    }
    let profile = fp.character.table().cpu_profile;
    for rule in profile.exclusions {
        let blocked =
            if rule.require_ground_or_stage && fp.core.physics.ground_or_air == GroundOrAir::Air {
                !fp.core.cpu.over_stage
            } else {
                blocked_or_no_floor_ahead(fp, scene, rule.range)
            };
        if blocked {
            exclude(fp, rule.scripts);
        }
    }
    if profile.copy_exclusions {
        copy_exclusions(fp, scene);
    }
}

/// `if (xEC < 8) { xCC[xEC] = script; xEC++; }` for each.
fn exclude(fp: &mut Fighter, list: &[i32]) {
    for &script in list {
        fp.core.cpu.excluded_attacks.push(script);
    }
}

/// ftCo_800B77E8's FTKIND_KIRBY arm (800B8734): airborne off stage he
/// drops his special; with Donkey Kong's, Samus's or Mewtwo's charged
/// ability (u.kb.hat.kind, 0x2238) he drops the Inhale; otherwise its other
/// form, and the Inhale too against a bigger target (x34_scale.y).
fn copy_exclusions(fp: &mut Fighter, scene: &Scene) {
    if fp.core.physics.ground_or_air == GroundOrAir::Air && !fp.core.cpu.over_stage {
        exclude(fp, &[scripts::SPECIAL]);
    }
    if (fp.character.table().cpu_copied_charge)(&fp.character) {
        exclude(fp, &[scripts::INHALE]);
        return;
    }
    exclude(fp, &[scripts::INHALE_OTHER]);
    let target = fp.core.cpu.target.map(|index| scene.fighter(index));
    let target_scale = target
        .expect("ftCo_800B77E8 runs for ftCo_800B8A9C's target")
        .player
        .scale;
    // 800B8808: fcmpo, bge.
    if fp.player.scale < target_scale {
        exclude(fp, &[scripts::INHALE]);
    }
}

/// ftCo_CpuCanUseRangedAttack (inlined, ftcpuattack.c:1381): a wall or
/// floor between the CPU and its target (mpCheckAll), or no usable floor
/// `range` ahead of it (ftCo_800A0FB0 from 5 above to 1000 below; the x an
/// fadds/fsubs, 800B78FC/800B795C).
fn blocked_or_no_floor_ahead(fp: &Fighter, scene: &mut Scene, range: f32) -> bool {
    let p = fp.core.physics.position;
    if let Some(target) = fp.core.cpu.target {
        let t = scene.fighter(target).core.physics.position;
        if scene.map.check_all(-1, -1, p.x, p.y, t.x, t.y).is_some() {
            return true;
        }
    }
    // 800B78CC: fcmpo with 0.0 in double.
    let x = if f64::from(fp.core.physics.facing) > 0.0 {
        p.x + range
    } else {
        p.x - range
    };
    let above = (PROBE_ABOVE + f64::from(p.y)) as f32;
    let below = (f64::from(p.y) - PROBE_BELOW) as f32;
    scene.check_usable_floor(x, above, x, below).is_none()
}
