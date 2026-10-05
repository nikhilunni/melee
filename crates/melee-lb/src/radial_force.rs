//! The eight-entry dynamics force-field pool (lb_00F9.c, `lb_80011A50_t`):
//! radial impulses (lb_800119DC, priority 100) and directional stage gusts
//! (lb_80011A50, priority 0), allocated by lb_800100B0 and aged by
//! lb_800115F4, which also derives the wind state lb_80011ABC reports.
//!
//! New entries prepend (the free list feeds the active head). At capacity
//! lb_800100B0 reuses the first entry of maximal priority in place, unless
//! that priority is below the request's.
use crate::dynamics::ForceField;
use hsd_types::Vec3;

/// lb_800119DC's priority (`x1`).
const RADIAL_PRIORITY: i32 = 100;
/// lb_80011A50's priority (`x1`).
const DIRECTIONAL_PRIORITY: i32 = 0;
/// lb_800115F4: directional strength above this sum counts as windy.
const WINDY_STRENGTH: f64 = 0.1;

#[derive(Clone, Copy, Debug)]
pub struct RadialImpulse {
    pub center: Vec3,
    pub frames: i32,
    pub strength: f32,
    pub decay: f32,
    pub phase_step: f32,
}

/// lb_80011A50 (0x80011A50): a directional push inside a rectangle.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalGust {
    pub direction: Vec3,
    /// Lifetime in pool ticks; negative never expires.
    pub frames: i32,
    pub strength: f32,
    pub decay: f32,
    pub phase_step: f32,
    /// `[left, top, right, bottom]` (x10, x14, x18, x1C), strict bounds.
    pub rectangle: [f32; 4],
}

/// lb_804D63B4 as lb_80011ABC returns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindState {
    #[default]
    Calm,
    /// The first windy pool tick (1).
    Started,
    /// Windy again (2).
    Blowing,
    /// The first calm tick after wind (-1).
    Stopped,
}

impl WindState {
    /// lb_80011ABC (0x80011ABC) as an integer.
    pub fn code(self) -> i32 {
        match self {
            Self::Calm => 0,
            Self::Started => 1,
            Self::Blowing => 2,
            Self::Stopped => -1,
        }
    }
}

#[derive(Default, Clone)]
pub struct RadialForces {
    fields: [ForceField; 8],
    timers: [i32; 8],
    decay: [f32; 8],
    priority: [i32; 8],
    len: usize,
    wind: WindState,
}

impl RadialForces {
    /// lb_800119DC (0x800119DC).
    pub fn insert(&mut self, impulse: RadialImpulse) {
        let field = ForceField {
            direction_or_center: impulse.center,
            rectangle: None,
            strength: impulse.strength,
            phase: 0,
            phase_step: impulse.phase_step,
        };
        self.allocate(field, impulse.frames, impulse.decay, RADIAL_PRIORITY);
    }

    /// lb_80011A50 (0x80011A50).
    pub fn insert_directional(&mut self, gust: DirectionalGust) {
        let field = ForceField {
            direction_or_center: gust.direction,
            rectangle: Some(gust.rectangle),
            strength: gust.strength,
            phase: 0,
            phase_step: gust.phase_step,
        };
        self.allocate(field, gust.frames, gust.decay, DIRECTIONAL_PRIORITY);
    }

    /// lb_800100B0 (0x800100B0).
    fn allocate(&mut self, field: ForceField, frames: i32, decay: f32, priority: i32) {
        let slot = if self.len < 8 {
            self.fields.copy_within(0..self.len, 1);
            self.timers.copy_within(0..self.len, 1);
            self.decay.copy_within(0..self.len, 1);
            self.priority.copy_within(0..self.len, 1);
            self.len += 1;
            0
        } else {
            // inlineA1: the first entry of maximal priority.
            let mut slot = 0;
            for i in 1..self.len {
                if self.priority[i] > self.priority[slot] {
                    slot = i;
                }
            }
            if self.priority[slot] < priority {
                return;
            }
            slot
        };
        self.fields[slot] = field;
        self.timers[slot] = frames;
        self.decay[slot] = decay;
        self.priority[slot] = priority;
    }

    pub fn fields(&self) -> &[ForceField] {
        &self.fields[..self.len]
    }

    /// lb_80011ABC (0x80011ABC).
    pub fn wind_state(&self) -> WindState {
        self.wind
    }

    /// lb_800115F4 (0x800115F4), run by the stage controller: sum directional
    /// strength, subtract decay, clamp, decrement a positive timer, advance
    /// phase and unlink expired entries in traversal order; then update the
    /// wind state from the sum taken before decay.
    pub fn tick(&mut self) {
        let mut directional = 0.0_f32;
        let mut i = 0;
        while i < self.len {
            if self.fields[i].rectangle.is_some() {
                directional += self.fields[i].strength;
            }
            // retail 0x80011630: fcmpo strength, 0; bge.
            self.fields[i].strength =
                gekko_math::cmp::max(self.fields[i].strength - self.decay[i], 0.0);
            if self.timers[i] > 0 {
                self.timers[i] -= 1;
            }
            self.fields[i].phase = self.fields[i].phase.wrapping_add(1);
            if self.timers[i] == 0 {
                self.fields.copy_within(i + 1..self.len, i);
                self.timers.copy_within(i + 1..self.len, i);
                self.decay.copy_within(i + 1..self.len, i);
                self.priority.copy_within(i + 1..self.len, i);
                self.len -= 1;
            } else {
                i += 1;
            }
        }
        let was_windy = self.wind.code() > 0;
        self.wind = match (f64::from(directional) > WINDY_STRENGTH, was_windy) {
            (true, true) => WindState::Blowing,
            (true, false) => WindState::Started,
            (false, true) => WindState::Stopped,
            (false, false) => WindState::Calm,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radial(x: f32) -> RadialImpulse {
        RadialImpulse {
            center: Vec3::new(x, 0.0, 0.0),
            frames: 2,
            strength: 1.0,
            decay: 0.25,
            phase_step: 1.0,
        }
    }

    fn gust(frames: i32) -> DirectionalGust {
        DirectionalGust {
            direction: Vec3::new(-1.0, 0.0, 0.0),
            frames,
            strength: 0.5,
            decay: 0.0,
            phase_step: 0.0,
            rectangle: [-74.0, 40.0, -18.0, -10.0],
        }
    }

    #[test]
    fn retail_pool_replaces_head_and_expires_in_order() {
        let mut pool = RadialForces::default();
        for n in 0..9 {
            pool.insert(radial(n as f32));
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

    #[test]
    fn full_pool_of_gusts_rejects_radial_but_gusts_replace_radials() {
        let mut pool = RadialForces::default();
        for _ in 0..8 {
            pool.insert_directional(gust(15));
        }
        pool.insert(radial(1.0));
        assert!(pool.fields().iter().all(|f| f.rectangle.is_some()));

        let mut pool = RadialForces::default();
        pool.insert(radial(1.0));
        for _ in 0..7 {
            pool.insert_directional(gust(15));
        }
        // The only priority-100 entry is last in list order.
        pool.insert_directional(gust(15));
        assert!(pool.fields().iter().all(|f| f.rectangle.is_some()));
    }

    #[test]
    fn wind_state_follows_directional_strength_before_decay() {
        let mut pool = RadialForces::default();
        pool.insert(radial(0.0));
        pool.tick();
        assert_eq!(pool.wind_state(), WindState::Calm);
        pool.insert_directional(gust(2));
        let states: Vec<i32> = (0..4)
            .map(|_| {
                pool.tick();
                pool.wind_state().code()
            })
            .collect();
        assert_eq!(states, [1, 2, -1, 0]);
    }
}
