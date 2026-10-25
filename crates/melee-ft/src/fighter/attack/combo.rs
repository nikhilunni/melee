//! Repeated-hit separation, ftColl_800763C0 / ftColl_80076528.
use crate::fighter::{assets::Result, FighterCore};
use hsd_archive::Archive;
use melee_types::combat::StaleMove as GroundMove;
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
#[derive(Default, Clone)]
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
    /// ftColl_80076528 (80076528): runs after ordinary physics, including
    /// hitlag, but before the moving-floor offset (fighter.c:2366-2380). The
    /// push is decided here, before the tick's physics, and applied by
    /// `integrate_environment` (or by `finish_combo_push` when no
    /// environment step runs).
    pub fn prepare_combo_push(&mut self, p: &ComboParameters) {
        let combo = &mut self.combat.combo;
        if combo.remaining == 0 {
            return;
        }
        combo.remaining -= 1;
        if self.combat.grab.is_some() || self.physics.ground_or_air != GroundOrAir::Ground {
            return;
        }
        let step = p.steps[usize::from(combo.count >= p.stronger_threshold)];
        self.physics.combo_push = Some(crate::physics::ComboPush {
            normal: self.collision.data.floor.normal,
            step,
        });
    }
    /// The push of a tick whose physics ran no environment step.
    pub fn finish_combo_push(&mut self) {
        if let Some(push) = self.physics.combo_push.take() {
            if self.physics.ground_or_air == GroundOrAir::Ground {
                push.apply(&mut self.physics.position, self.physics.facing);
            }
        }
    }
}
