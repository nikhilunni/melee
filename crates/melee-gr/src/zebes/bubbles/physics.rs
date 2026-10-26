//! grZebes_801DB3CC (0x801DB3CC): each bubble's update (grZebes_801DB088),
//! then the forces of one tick: every bubble sinks, the chain's links
//! pull toward their neighbours, every pair pushes apart or attracts, and
//! the loose bubbles move inside the two parts' bounds.
use super::{
    is_anchor, BubbleEngine, BubbleState, Bubbles, BUBBLES, CHAIN, LEFT_ANCHOR, RIGHT_ANCHOR,
};
use crate::zebes::Parameters;
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_types::Vec3;

/// A burst's particle is 4 in front of the bubble.
const BURST_DEPTH: f32 = 4.0;
/// A burst chain link takes the nearest loose bubble within 8 (64 squared).
const REPLACEMENT_REACH: f32 = 64.0;
/// A dropping bubble waits `HSD_Randi(0x3C) + 0xA` ticks.
const DROP_WAIT: (i32, i16) = (0x3C, 0xA);
/// Two bubbles closer than this push along +y (the double 0.001).
const MIN_DISTANCE: f64 = 0.001;
/// The float the push then uses for that distance.
const MIN_DISTANCE_F32: f32 = 0.001;

/// `dx * dx + dy * dy` as the chain's inline sqrtf reads it (fmuls,
/// fmadds: 0x801DB610).
fn length(dx: f32, dy: f32) -> f32 {
    sqrtf(fmadds(dx, dx, dy * dy))
}

impl Bubbles {
    /// grZebes_801DB3CC. Returns whether a burst left the chain without a
    /// replacement (the top platform then sinks).
    pub fn update(
        &mut self,
        parameters: &Parameters,
        pulse_ticks: u32,
        engine: &mut impl BubbleEngine,
    ) -> bool {
        let mut broken = false;
        for index in 0..BUBBLES {
            if self.entries[index].state != BubbleState::Free
                && self.update_one(index, parameters, pulse_ticks, engine)
            {
                broken = true;
            }
        }
        for bubble in self.entries.iter_mut() {
            if bubble.state != BubbleState::Free {
                bubble.vx = 0.0;
                bubble.vy = -parameters.sink;
            }
        }
        self.pull_chain(parameters, engine);
        self.push_pairs(parameters);
        self.r#move(parameters);
        broken
    }

    /// grZebes_801DB088 (0x801DB088) on a bubble with a model: grow toward
    /// its size, drop the pulse model whose animation ended, burst.
    fn update_one(
        &mut self,
        index: usize,
        parameters: &Parameters,
        pulse_ticks: u32,
        engine: &mut impl BubbleEngine,
    ) -> bool {
        let bubble = &mut self.entries[index];
        if bubble.object.is_none() {
            return false;
        }
        // bgp->u.zebes3.xC4 (a material item) is never set: no hits toggle.
        let speed = parameters.growth;
        let difference = bubble.target_size - bubble.size;
        if difference > speed {
            bubble.size += speed;
        } else if difference < -speed {
            bubble.size -= speed;
        } else {
            bubble.size = bubble.target_size;
        }
        // grAnime_801C83D0(x20_gobj, 0, 7): the pulse model is the first
        // animated part of the bubble's model.
        if bubble.pulse.is_some_and(|age| age >= pulse_ticks) {
            bubble.pulse = None;
        }
        match bubble.state {
            BubbleState::Dropping => {
                let timer = bubble.timer;
                bubble.timer = timer.wrapping_sub(1);
                if timer < 0 {
                    self.burst(index, engine);
                    // Ground_801C5440(gp, 0, 400006): a sound.
                }
                false
            }
            BubbleState::Bursting => {
                self.burst(index, engine);
                index < CHAIN && !is_anchor(index) && self.replace_link(index, engine)
            }
            _ => false,
        }
    }

    fn burst(&mut self, index: usize, engine: &mut impl BubbleEngine) {
        let bubble = &mut self.entries[index];
        let object = bubble.object.take().expect("a bubble with a model");
        engine.burst(object, Vec3::new(bubble.x, bubble.y, BURST_DEPTH));
        bubble.pulse = None;
        bubble.state = BubbleState::Free;
    }

    /// A burst chain link: the nearest held loose bubble within reach takes
    /// its place; with none, every held loose bubble drops. Returns whether
    /// none was in reach.
    fn replace_link(&mut self, index: usize, engine: &mut impl BubbleEngine) -> bool {
        let (x, y) = (self.entries[index].x, self.entries[index].y);
        let mut best = None;
        let mut best_distance = REPLACEMENT_REACH;
        for candidate in CHAIN..BUBBLES {
            let other = &self.entries[candidate];
            if other.is_held() {
                // retail: fsubs, fsubs, fmuls, fmuls, fadds.
                let dx = other.x - x;
                let dy = other.y - y;
                let distance = dx * dx + dy * dy;
                if distance < best_distance {
                    best = Some(candidate);
                    best_distance = distance;
                }
            }
        }
        match best {
            Some(candidate) => {
                let source = self.entries[candidate];
                let link = &mut self.entries[index];
                link.state = source.state;
                link.pulse = source.pulse;
                link.size = source.size;
                link.target_size = source.target_size;
                link.object = source.object;
                let source = &mut self.entries[candidate];
                source.state = BubbleState::Free;
                source.object = None;
                false
            }
            None => {
                for candidate in CHAIN..BUBBLES {
                    if self.entries[candidate].is_held() {
                        let wait = engine.rng().randi(DROP_WAIT.0) as i16 + DROP_WAIT.1;
                        let bubble = &mut self.entries[candidate];
                        bubble.state = BubbleState::Dropping;
                        bubble.timer = wait;
                    }
                }
                true
            }
        }
    }

    /// Each held or growing chain link pulls toward its neighbours (at most
    /// `max_pull` past the link length); a held link missing a neighbour
    /// drops.
    fn pull_chain(&mut self, parameters: &Parameters, engine: &mut impl BubbleEngine) {
        for index in 0..CHAIN {
            let state = self.entries[index].state;
            if !matches!(state, BubbleState::Held | BubbleState::Growing) || is_anchor(index) {
                continue;
            }
            let (mut fx, mut fy) = (0.0, 0.0);
            let mut neighbours = 0;
            for neighbour in [index - 1, index + 1] {
                if self.entries[neighbour].state == BubbleState::Free {
                    continue;
                }
                let (bubble, other) = (&self.entries[index], &self.entries[neighbour]);
                let dx = other.x - bubble.x;
                let dy = other.y - bubble.y;
                let distance = length(dx, dy);
                if distance > parameters.link {
                    // retail 0x801DB674..0x801DB694: fdivs, fsubs, fdivs,
                    // then fmadds into the sums.
                    let nx = dx / distance;
                    let mut pull = distance - parameters.link;
                    let ny = dy / distance;
                    if pull > parameters.max_pull {
                        pull = parameters.max_pull;
                    }
                    fx = fmadds(nx, pull, fx);
                    fy = fmadds(ny, pull, fy);
                }
                neighbours += 1;
            }
            let bubble = &mut self.entries[index];
            bubble.x += fx;
            bubble.y += fy;
            if bubble.state != BubbleState::Growing && neighbours != 2 {
                bubble.state = BubbleState::Dropping;
                bubble.timer = engine.rng().randi(DROP_WAIT.0) as i16 + DROP_WAIT.1;
            }
        }
    }

    /// Every pair: closer than half the link length times their sizes'
    /// sum, the loose ones are pushed apart (never down); between the near
    /// and far reaches, they attract.
    fn push_pairs(&mut self, parameters: &Parameters) {
        for a in 0..BUBBLES - 1 {
            if self.entries[a].state == BubbleState::Free {
                continue;
            }
            for b in a + 1..BUBBLES {
                if self.entries[b].state == BubbleState::Free {
                    continue;
                }
                let (first, second) = (self.entries[a], self.entries[b]);
                let mut dy = second.y - first.y;
                let mut dx = second.x - first.x;
                let mut distance = length(dx, dy);
                // retail 0x801DB890..0x801DB8A8: x/2 as fmuls by 0.5.
                let sizes = first.size + second.size;
                let contact = parameters.link * 0.5 * sizes;
                let near = parameters.attract_near * 0.5 * sizes;
                let far = parameters.attract_far * 0.5 * sizes;
                if f64::from(distance) < MIN_DISTANCE {
                    dy = MIN_DISTANCE_F32;
                    dx = 0.0;
                    distance = MIN_DISTANCE_F32;
                }
                if distance < contact {
                    // retail 0x801DB8C4..0x801DB8DC: fdivs, the double
                    // (x - 1.0) * 0.5 and * 0.9, each rounded.
                    let ratio = f64::from(contact / distance) - 1.0;
                    let strength = (ratio * 0.5) as f32;
                    let strength = (f64::from(strength) * 0.9) as f32;
                    if a >= CHAIN {
                        let vx = -dx * strength;
                        let vy = -dy * strength;
                        let bubble = &mut self.entries[a];
                        bubble.vx += vx;
                        bubble.vy += if vy < 0.0 { 0.0 } else { vy };
                    }
                    if b >= CHAIN {
                        let vy = dy * strength;
                        let vx = dx * strength;
                        let bubble = &mut self.entries[b];
                        bubble.vx += vx;
                        bubble.vy += if vy < 0.0 { 0.0 } else { vy };
                    }
                } else if distance >= near && distance < far {
                    let attraction = parameters.attraction / distance;
                    if a >= CHAIN {
                        let bubble = &mut self.entries[a];
                        bubble.vx += dx * attraction;
                        bubble.vy += dy * attraction;
                    }
                    if b >= CHAIN {
                        let bubble = &mut self.entries[b];
                        bubble.vx += -dx * attraction;
                        bubble.vy += -dy * attraction;
                    }
                }
            }
        }
    }

    /// The anchors sit on their points; the rest move by their push, kept
    /// out of the two parts below their bounds, and the push is capped.
    fn r#move(&mut self, parameters: &Parameters) {
        let [left_anchor, right_anchor] = self.points.anchors;
        let [left_bound, right_bound] = self.points.bounds;
        let top = parameters.max_speed;
        for (index, bubble) in self.entries.iter_mut().enumerate() {
            if bubble.state == BubbleState::Free {
                continue;
            }
            match index {
                LEFT_ANCHOR | RIGHT_ANCHOR => {
                    let anchor = if index == LEFT_ANCHOR {
                        left_anchor
                    } else {
                        right_anchor
                    };
                    bubble.vx = 0.0;
                    bubble.vy = 0.0;
                    bubble.x = anchor.x;
                    bubble.y = anchor.y;
                }
                _ => {
                    let mut x = bubble.x + bubble.vx;
                    let mut y = bubble.y + bubble.vy;
                    if y < left_bound.y && x < left_bound.x {
                        if bubble.y >= left_bound.y {
                            y = left_bound.y;
                        } else if bubble.x >= left_bound.x {
                            x = left_bound.x;
                        }
                    }
                    if y < right_bound.y && x > right_bound.x {
                        if bubble.y >= right_bound.y {
                            y = right_bound.y;
                        } else if bubble.x <= right_bound.x {
                            x = right_bound.x;
                        }
                    }
                    bubble.x = x;
                    bubble.y = y;
                    clamp(&mut bubble.vx, top);
                    clamp(&mut bubble.vy, top);
                }
            }
        }
    }
}

fn clamp(value: &mut f32, top: f32) {
    if *value > top {
        *value = top;
    } else if *value < -top {
        *value = -top;
    }
}
