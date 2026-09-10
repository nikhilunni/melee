//! Repeated-hit separation, ftColl_800763C0 / ftColl_80076528.
use super::stale::GroundMove;
use crate::fighter::{assets::Result, FighterCore};
use hsd_archive::Archive;
use melee_types::GroundOrAir;

pub struct ComboParameters {
    threshold: u16,
    stronger_threshold: u16,
    pub grace_frames: u16,
    steps: [f32; 2],
    push_frames: u16,
}
impl ComboParameters {
    pub fn read(archive: &Archive, root: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            threshold: r.u32(root + 0x4c4)? as u16,
            stronger_threshold: r.u32(root + 0x4c8)? as u16,
            grace_frames: r.u32(root + 0x4cc)? as u16,
            steps: [r.f32(root + 0x4d0)?, r.f32(root + 0x4d4)?],
            push_frames: r.u32(root + 0x4d8)? as u16,
        })
    }
}
#[derive(Default)]
pub struct ComboState {
    pub victim: Option<u32>,
    attack: Option<GroundMove>,
    count: u16,
    remaining: u16,
    pub grace: u16,
}
impl ComboState {
    /// ftColl_800763C0: a move change resets to zero, a new target starts at one.
    pub fn record(&mut self, victim: u32, attack: Option<GroundMove>, p: &ComboParameters) {
        if self.victim.is_none() {
            self.attack = attack;
            self.count = 1;
            self.victim = Some(victim);
        } else if self.victim == Some(victim) {
            if attack.is_some() && self.attack == attack {
                self.count = self.count.wrapping_add(1);
                if self.count >= p.threshold {
                    self.remaining = p.push_frames;
                }
            } else {
                self.count = 0;
                self.attack = attack;
            }
        }
    }
}
impl FighterCore {
    /// ftColl_80076528 (80076528): runs after ordinary physics, including hitlag.
    pub fn apply_combo_push(&mut self, p: &ComboParameters) {
        let combo = &mut self.combat.combo;
        if combo.remaining == 0 {
            return;
        }
        combo.remaining -= 1;
        if self.combat.grab.is_some() || self.physics.ground_or_air != GroundOrAir::Ground {
            return;
        }
        let step = p.steps[usize::from(combo.count >= p.stronger_threshold)] * self.physics.facing;
        let n = self.collision.data.floor.normal;
        // retail 8007658C / 800765A0: fnmsubs, including the outer negation.
        self.physics.position.x = gekko_math::fma::fnmsubs(n.y, step, self.physics.position.x);
        self.physics.position.y = gekko_math::fma::fnmsubs(-n.x, step, self.physics.position.y);
    }
}
