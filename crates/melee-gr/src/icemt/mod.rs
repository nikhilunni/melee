//! Icicle Mountain (gricemt.c, GrIm.dat, StKind 0x19).
//!
//! In Versus the field scrolls: map 9's controller (grIceMt_801F75FC)
//! now and then draws a speed, eases to it, and moves the two field
//! segments it holds (out of maps 1..6) down by it every frame. The
//! decomp calls them "under" and "upper": the upper one is the segment in
//! view and the under one waits above it. A segment scrolled a whole
//! segment's height away is removed and another made beyond the other.
pub mod procs;

use crate::greatbay::rand_range;
use gekko_math::{
    fma::{fmadds, fnmsubs},
    HsdRng,
};

/// `grIceMt_YakumonoParam` as Versus reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// x0: picks a field segment stays out of the draw once used.
    pub segment_cooldown: i16,
    /// x2: picks a speed stays out of the draw once used.
    pub speed_cooldown: i16,
    /// x4..x30: the twelve scroll speeds (`((f32*)(param + 4))[idx]`).
    pub speeds: [f32; 12],
    /// x34: frames an eased speed change takes.
    pub ease_frames: i16,
    /// x36 / x38: a speed's hold, `rand_range(x36, x38)`.
    pub hold: [i16; 2],
    /// x40: the gap between two segments, on top of their own extents.
    pub segment_gap: f32,
}

/// `icemt_field` (gricemt.c:221): each field segment's map and its
/// extents above (x4) and below (x8) its origin, before the map scale.
pub const FIELD: [(u8, f32, f32); 6] = [
    (1, 180.0, -180.0),
    (2, 190.0, -180.0),
    (3, 190.0, -195.0),
    (4, 195.0, -185.0),
    (5, 190.0, -200.0),
    (6, 180.0, -190.0),
];

/// The segment map at `icemt_field[index]`.
pub fn field_map(index: usize) -> u8 {
    FIELD[index].0
}

fn field_index(map: u8) -> usize {
    FIELD
        .iter()
        .position(|f| f.0 == map)
        .unwrap_or_else(|| panic!("gricemt.c:2717: map {map} is not a field segment"))
}

/// grIceMt_801F993C (0x801F993C): how far `under` stands above `upper`:
/// `-(scale * under.x8 - (scale * upper.x4 + x40))` (retail 0x801F9AA0
/// fmadds, 0x801F9AA4 fnmsubs).
pub fn spacing(parameters: &Parameters, scale: f32, under: u8, upper: u8) -> f32 {
    let below = FIELD[field_index(under)].2;
    let above = FIELD[field_index(upper)].1;
    fnmsubs(scale, below, fmadds(scale, above, parameters.segment_gap))
}

/// `grIceMt_FA364_State`: the scroll speed and its easing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scroll {
    /// 0: holding a speed; 1: easing to the next.
    pub phase: i16,
    /// The hold's countdown; a pick when it passes zero.
    pub delay: i16,
    /// The easing's frames left.
    pub ease: i16,
    /// The speed eased to.
    pub index: i16,
    /// The speed now.
    pub speed: f32,
}

impl Default for Scroll {
    fn default() -> Self {
        Self {
            phase: 0,
            delay: 0,
            ease: 0,
            index: -1,
            speed: 0.0,
        }
    }
}

/// Map 9's controller (`grIceMt_GObj9_GroundVars`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controller {
    /// ids.under / ids.upper: the field segments' maps, -1 for none.
    pub under: i16,
    pub upper: i16,
    pub scroll: Scroll,
    /// x14: still waiting for the match to start (fn_801F75EC clears it).
    pub waiting: bool,
    /// x18: each speed's picks left out of the draw.
    pub speed_cooldowns: [i16; 12],
    /// x30: each field segment's picks left out of the draw.
    pub field_cooldowns: [i16; 6],
}

impl Controller {
    /// stageGObj9_OnInit (0x801F75FC): the first two segments, each a
    /// HSD_Randi(6) drawn until one not cooling down.
    pub fn new(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let mut controller = Self {
            under: -1,
            upper: -1,
            scroll: Scroll::default(),
            waiting: true,
            speed_cooldowns: [0; 12],
            field_cooldowns: [0; 6],
        };
        let under = controller.first_pick(parameters, rng);
        let upper = controller.first_pick(parameters, rng);
        controller.under = i16::from(field_map(under));
        controller.upper = i16::from(field_map(upper));
        controller
    }

    fn first_pick(&mut self, parameters: &Parameters, rng: &mut HsdRng) -> usize {
        loop {
            let index = rng.randi(6) as usize;
            if self.field_cooldowns[index] == 0 {
                self.field_cooldowns[index] = parameters.segment_cooldown;
                return index;
            }
        }
    }

    /// fn_801F9038 (0x801F9038): the next segment, drawn until one neither
    /// cooling down nor held; every cooldown then counts down.
    pub fn pick_segment(&mut self, parameters: &Parameters, rng: &mut HsdRng) -> u8 {
        let index = loop {
            let index = rng.randi(6) as usize;
            let map = i16::from(field_map(index));
            if self.field_cooldowns[index] == 0 && self.under != map && self.upper != map {
                break index;
            }
        };
        for cooldown in &mut self.field_cooldowns {
            if *cooldown > 0 {
                *cooldown -= 1;
            }
        }
        self.field_cooldowns[index] = parameters.segment_cooldown;
        field_map(index)
    }

    /// fn_801F8E58 (0x801F8E58): a speed out of those not cooling down,
    /// every cooldown counting down, and its hold
    /// (grIceMt_GetRandomTimer: `rand_range(x36, x38)`).
    fn pick_speed(&mut self, parameters: &Parameters, rng: &mut HsdRng) -> (i16, i16) {
        let mut open = [0usize; 12];
        let mut count = 0;
        for (index, &cooldown) in self.speed_cooldowns.iter().enumerate() {
            // Stage_80225194() != 212: every speed is open in Versus.
            if cooldown == 0 {
                open[count] = index;
                count += 1;
            }
        }
        assert!(count != 0, "gricemt.c:2077: no speed left to draw");
        let chosen = open[rng.randi(count as i32) as usize];
        for cooldown in &mut self.speed_cooldowns {
            if *cooldown > 0 {
                *cooldown -= 1;
            }
        }
        self.speed_cooldowns[chosen] = parameters.speed_cooldown;
        let [a, b] = parameters.hold;
        let hold = rand_range(i32::from(a), i32::from(b), rng);
        (chosen as i16, hold as i16)
    }

    /// grIceMt_801FA364 (0x801FA364) with fn_801F8E58: the hold counts
    /// down, a pick past it starts an ease of x34 frames (retail
    /// 0x801FA444..0x801FA450: fsubs, the count as a float, fdivs, fadds).
    /// The burst (x3C) is Adventure's. Returns the speed.
    pub fn step_scroll(&mut self, parameters: &Parameters, rng: &mut HsdRng) -> f32 {
        let scroll = &mut self.scroll;
        match scroll.phase {
            0 => {
                let delay = scroll.delay;
                scroll.delay = delay.wrapping_sub(1);
                if delay < 0 {
                    let (index, hold) = self.pick_speed(parameters, rng);
                    let scroll = &mut self.scroll;
                    scroll.index = index;
                    scroll.delay = hold;
                    scroll.phase = 1;
                    scroll.ease = parameters.ease_frames;
                }
            }
            1 => {
                scroll.ease -= 1;
                let target = parameters.speeds[scroll.index as usize];
                if scroll.ease != 0 {
                    scroll.speed += (target - scroll.speed) / f32::from(scroll.ease);
                } else {
                    scroll.phase = 0;
                    scroll.speed = target;
                }
            }
            _ => {}
        }
        self.scroll.speed
    }
}
