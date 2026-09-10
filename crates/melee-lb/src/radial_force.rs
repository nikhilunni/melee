//! lb_800119DC, lb_800100B0 and lb_800115F4: eight radial dynamics fields.
//! Every radial request has priority 100. New entries prepend; at capacity the
//! first maximum-priority entry (the head for this family) is replaced in place.
use crate::dynamics::ForceField;
use hsd_types::Vec3;
#[derive(Clone, Copy, Debug)]
pub struct RadialImpulse {
    pub center: Vec3,
    pub frames: i32,
    pub strength: f32,
    pub decay: f32,
    pub phase_step: f32,
}
#[derive(Default)]
pub struct RadialForces {
    fields: [ForceField; 8],
    timers: [i32; 8],
    decay: [f32; 8],
    len: usize,
}
impl RadialForces {
    pub fn insert(&mut self, impulse: RadialImpulse) {
        if self.len < 8 {
            self.fields.copy_within(0..self.len, 1);
            self.timers.copy_within(0..self.len, 1);
            self.decay.copy_within(0..self.len, 1);
            self.len += 1;
        }
        self.fields[0] = ForceField {
            direction_or_center: impulse.center,
            rectangle: None,
            strength: impulse.strength,
            phase: 0,
            phase_step: impulse.phase_step,
        };
        self.timers[0] = impulse.frames;
        self.decay[0] = impulse.decay;
    }
    pub fn fields(&self) -> &[ForceField] {
        &self.fields[..self.len]
    }
    /// Ground controller tail: subtract decay, clamp, decrement positive timer,
    /// advance phase, then unlink expired entries, preserving traversal order.
    pub fn tick(&mut self) {
        let mut i = 0;
        while i < self.len {
            self.fields[i].strength = (self.fields[i].strength - self.decay[i]).max(0.0);
            if self.timers[i] > 0 {
                self.timers[i] -= 1;
            }
            self.fields[i].phase = self.fields[i].phase.wrapping_add(1);
            if self.timers[i] == 0 {
                self.fields.copy_within(i + 1..self.len, i);
                self.timers.copy_within(i + 1..self.len, i);
                self.decay.copy_within(i + 1..self.len, i);
                self.len -= 1;
            } else {
                i += 1;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retail_pool_replaces_head_and_expires_in_order() {
        let mut pool = RadialForces::default();
        for n in 0..9 {
            pool.insert(RadialImpulse {
                center: Vec3::new(n as f32, 0.0, 0.0),
                frames: 2,
                strength: 1.0,
                decay: 0.25,
                phase_step: 1.0,
            });
        }
        assert_eq!(pool.fields().len(), 8);
        assert_eq!(pool.fields()[0].direction_or_center.x, 8.0);
        assert_eq!(pool.fields()[1].direction_or_center.x, 6.0);
        pool.tick();
        assert_eq!(pool.fields()[0].strength, 0.75);
        assert_eq!(pool.fields()[0].phase, 1);
        pool.tick();
        assert!(pool.fields().is_empty());
    }
}
