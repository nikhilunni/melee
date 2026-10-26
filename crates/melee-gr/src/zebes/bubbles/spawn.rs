//! Where bubbles come from: grZebes_801DAE70 (one bubble), grZebes_801DC744
//! (the chain and the first loose bubbles), grZebes_801DC9DC (the stage's
//! creation), grZebes_801DC408 (a loose bubble every few ticks), and how
//! they go: fn_801DAC90 (a hit bursts one) and grZebes_801DAA08 (a pulse).
use super::{
    is_anchor, Bubble, BubbleEngine, BubbleState, Bubbles, Points, BUBBLES, CHAIN, INITIAL_POINTS,
};
use crate::zebes::Parameters;
use gekko_math::fma::{fmadd, fmadds, fnmsubs};

/// grZe_803B8044's chain sizes.
pub const CHAIN_SIZES: [f32; CHAIN] = [1.0, 1.1, 1.0, 1.2, 1.1, 1.0, 1.0];
/// grZebes_801DC408 grows no bubble while this many are out.
const MOST_BUBBLES: i32 = 15;
/// A loose bubble starts this small (the float 0.001).
const SEED_SIZE: f32 = 0.001;
/// fn_801DAC90: a burst holds the loose bubbles back this many ticks.
const BURST_COOLDOWN: i32 = 0x78;
/// The double 0.001 a hit bubble shrinks toward.
const BURST_SIZE: f64 = 0.001;
/// grZebes_801DC744's mask bits: the chain, the loose bubbles.
pub const CHAIN_LINE: u32 = 1;
pub const LOOSE_BUBBLES: u32 = 2;
/// The loose bubbles grZebes_801DC744 places at creation: (x fraction, y
/// fraction or `None` for the full height, size). The fractions are
/// doubles (0.2, 0.5, 0.6, 0.8, 0.9).
const LOOSE: [(f64, Option<f64>, f32); 9] = [
    (0.5, Some(0.5), 1.0),
    (0.2, Some(0.8), 1.2),
    (0.8, Some(0.2), 1.1),
    (0.8, Some(0.8), 1.1),
    (0.5, Some(0.2), 1.2),
    (0.5, Some(0.9), 1.3),
    (0.5, Some(0.9), 1.3),
    (0.6, None, 1.1),
    (0.2, None, 1.0),
];

impl Bubbles {
    /// grZebes_801DC9DC (0x801DC9DC), at the stage's creation: every slot
    /// free, the four points from grZe_803B8044, the chain and the loose
    /// bubbles, then a hundred ticks of their forces.
    pub fn create(parameters: &Parameters, engine: &mut impl BubbleEngine) -> Self {
        let mut bubbles = Self {
            entries: [Bubble::FREE; BUBBLES],
            points: INITIAL_POINTS,
            cooldown: 0,
        };
        bubbles.spawn_lines(
            CHAIN_LINE | LOOSE_BUBBLES,
            BubbleState::Held,
            parameters,
            engine,
        );
        for _ in 0..100 {
            // The pulse model never exists yet: its lifetime is moot.
            bubbles.update(parameters, u32::MAX, engine);
        }
        bubbles
    }

    /// grZebes_801DAE70 (0x801DAE70): a bubble in a free slot without a
    /// model. The anchors get none.
    pub fn spawn_at(
        &mut self,
        index: usize,
        state: BubbleState,
        x: f32,
        y: f32,
        size: f32,
        engine: &mut impl BubbleEngine,
    ) {
        let bubble = &mut self.entries[index];
        if bubble.state != BubbleState::Free || bubble.object.is_some() {
            return;
        }
        bubble.x = x;
        bubble.y = y;
        bubble.vx = 0.0;
        bubble.vy = 0.0;
        bubble.target_size = size;
        bubble.size = size;
        // grZebes_801D8558(3) places the model at (x, y, 0) scaled
        // (size, size, 0.001); zebes3.xC4 = NULL, xC8 = index.
        let object = if is_anchor(index) {
            None
        } else {
            Some(engine.create())
        };
        let bubble = &mut self.entries[index];
        bubble.state = state;
        bubble.object = object;
        bubble.pulse = None;
    }

    /// grZebes_801DC744 (0x801DC744).
    pub fn spawn_lines(
        &mut self,
        mask: u32,
        state: BubbleState,
        parameters: &Parameters,
        engine: &mut impl BubbleEngine,
    ) {
        let Points { bounds, anchors } = self.points;
        if mask & CHAIN_LINE != 0 {
            let start = anchors[0];
            // retail 0x801DC7F8 / 0x801DC808: fdivs by 6.0f.
            let step_x = (anchors[1].x - start.x) / 6.0;
            let step_y = (anchors[1].y - start.y) / 6.0;
            for (index, size) in CHAIN_SIZES.into_iter().enumerate() {
                // retail 0x801DC83C / 0x801DC840: fmadds.
                let x = fmadds(step_x, index as f32, start.x);
                let y = fmadds(step_y, index as f32, start.y);
                self.spawn_at(index, state, x, y, size, engine);
            }
        }
        if mask & LOOSE_BUBBLES != 0 {
            let start = anchors[0];
            let radius = parameters.link;
            let base_x = start.x + radius;
            // retail 0x801DC890: fnmsubs, (right bound - start) - 2r.
            let width = fnmsubs(2.0, radius, bounds[1].x - start.x);
            let height = bounds[1].y - start.y;
            for (offset, (fx, fy, size)) in LOOSE.into_iter().enumerate() {
                // retail 0x801DC89C..: fmadd in double, frsp.
                let x = fmadd(fx, f64::from(width), f64::from(base_x)) as f32;
                let y = match fy {
                    Some(fy) => fmadd(fy, f64::from(height), f64::from(start.y)) as f32,
                    // retail 0x801DC96C: fadd in double.
                    None => (f64::from(start.y) + f64::from(height)) as f32,
                };
                self.spawn_at(CHAIN + offset, state, x, y, size, engine);
            }
        }
    }

    /// grZebes_801DC408 (0x801DC408) while the top platform stands: after
    /// the burst cooldown, a loose bubble every `bubble_wait` ticks while
    /// fewer than fifteen are out. `timer` is map 6's xF4.
    pub fn grow_loose(
        &mut self,
        timer: &mut i16,
        parameters: &Parameters,
        engine: &mut impl BubbleEngine,
    ) {
        if self.cooldown != 0 {
            self.cooldown -= 1;
            return;
        }
        *timer = timer.wrapping_sub(1);
        if *timer >= 0 {
            return;
        }
        let mut free = None;
        let mut count = 0;
        for (index, bubble) in self.entries.iter().enumerate() {
            if bubble.state != BubbleState::Free {
                count += 1;
            } else if free.is_none() {
                free = Some(index);
            }
        }
        if count < MOST_BUBBLES {
            let free = free.expect("fewer than fifteen bubbles leave a free slot");
            let [left_anchor, _] = self.points.anchors;
            let right_bound = self.points.bounds[1];
            let radius = parameters.link;
            // retail 0x801DC630 fadds, 0x801DC634 fnmsubs, 0x801DC638 fsubs.
            let base_x = left_anchor.x + radius;
            let width = fnmsubs(2.0, radius, right_bound.x - left_anchor.x);
            let height = right_bound.y - left_anchor.y;
            let rng = engine.rng();
            let size_draw = rng.randf();
            let y_draw = rng.randf();
            let x_draw = rng.randf();
            // retail 0x801DC654 / 0x801DC65C fmadds, 0x801DC664 fmadd.
            let x = fmadds(width, x_draw, base_x);
            let y = fmadds(height, y_draw, left_anchor.y);
            let size = fmadd(0.5, f64::from(size_draw), 1.0) as f32;
            self.spawn_at(free, BubbleState::Held, x, y, size, engine);
            self.entries[free].size = SEED_SIZE;
            // Ground_801C53EC(400003): a sound.
        }
        let [low, high] = parameters.bubble_wait;
        let low = gekko_math::msl::fctiwz(low);
        let high = gekko_math::msl::fctiwz(high);
        *timer = crate::onett::rand_range(high, low, engine.rng()) as i16;
    }

    /// fn_801DAC90: a hit on the bubbles' material item bursts the first
    /// held bubble that is not an anchor, whichever was struck.
    pub fn hit(&mut self) {
        if let Some(index) =
            (0..BUBBLES).find(|&index| self.entries[index].is_held() && !is_anchor(index))
        {
            let bubble = &mut self.entries[index];
            bubble.state = BubbleState::Bursting;
            bubble.target_size = BURST_SIZE as f32;
            self.cooldown = BURST_COOLDOWN;
        }
    }

    /// grZebes_801DAA08 (0x801DAA08): a pulse on a random held bubble that
    /// is not an anchor and has none. Returns whether one started.
    pub fn pulse(&mut self, engine: &mut impl BubbleEngine) -> bool {
        let mut candidates = [0usize; BUBBLES];
        let mut count = 0;
        for (index, bubble) in self.entries.iter().enumerate() {
            if bubble.is_held() && !is_anchor(index) && bubble.pulse.is_none() {
                candidates[count] = index;
                count += 1;
            }
        }
        if count == 0 {
            return false;
        }
        let chosen = candidates[engine.rng().randi(count as i32) as usize];
        // Ground_801C13D0(2, 0): map 2's model under the bubble's, which
        // it hides; its animation requested at 0 and evaluated once.
        self.entries[chosen].pulse = Some(0);
        true
    }

    /// The pulse models' animations advance (each bubble model's
    /// Ground_801C1CD0 at s_link 1).
    pub fn age_pulses(&mut self) {
        for bubble in self.entries.iter_mut() {
            if bubble.state != BubbleState::Free && bubble.object.is_some() {
                if let Some(age) = &mut bubble.pulse {
                    *age = age.saturating_add(1);
                }
            }
        }
    }
}
