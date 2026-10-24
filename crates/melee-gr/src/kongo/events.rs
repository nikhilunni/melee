//! grKongo_801D7BBC (0x801D7BBC): the stage's random events.
use super::{Kongo, Parameters};
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::fctiwz,
    HsdRng,
};

/// What an event creates (grKongo_801D5340 of a map).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// 1: a path rider (map 11) carrying a Klaptrap (it_802E18B4).
    Klaptrap,
    /// 2: a path rider carrying a box (it_80286088); items on only.
    Box,
    /// 3: a stage object: a log (maps 1, 2) or a passer-by (maps 7, 8, 9).
    Object(u8),
}
impl Event {
    fn kind(self) -> i16 {
        match self {
            Self::Klaptrap => 1,
            Self::Box => 2,
            Self::Object(_) => 3,
        }
    }
}

impl Kongo {
    /// grKongo_801D7BBC: count the event timer down; at zero draw the next
    /// wait and an event. `items` is gm_8016B238() == 0 &&
    /// it_8026D324(It_Kind_Box) (the box's item switch).
    pub fn tick_events(&mut self, items: bool, rng: &mut HsdRng) -> Option<Event> {
        if self.event_timer > 0 {
            self.event_timer -= 1;
        }
        if self.event_timer != 0 {
            return None;
        }
        let p: &Parameters = &self.parameters;
        let box_weight = if items { p.box_weight } else { 0.0 };
        // 801D7C54..801D7C68: fadds, fmadds, fmadds, then times the draw.
        let r = rng.randf();
        let total = fmadds(
            p.object_weight,
            1.0 - p.log_share,
            fmadds(p.object_weight, p.log_share, p.klap_weight + box_weight),
        );
        let mut roll = r * total;
        let r = rng.randf();
        let [low, high] = p.event_wait;
        self.event_timer = fctiwz(fmadds(high - low, r, low)) as i16;
        roll -= p.klap_weight;
        let event = if roll < 0.0 {
            Event::Klaptrap
        } else {
            roll -= box_weight;
            if roll < 0.0 {
                Event::Box
            } else {
                roll -= p.object_weight * p.log_share;
                if roll < 0.0 {
                    Event::Object(if rng.randi(2) != 0 { 1 } else { 2 })
                } else {
                    // 801D7D50: fnmsubs, roll - x10 * (1 - x18).
                    roll = fnmsubs(p.object_weight, 1.0 - p.log_share, roll);
                    if roll >= 0.0 {
                        return None;
                    }
                    Event::Object(match rng.randi(3) {
                        0 => 7,
                        1 => 8,
                        _ => 9,
                    })
                }
            }
        };
        if self.last_event == event.kind() && rng.randf() > self.parameters.repeat_chance {
            return None;
        }
        self.last_event = event.kind();
        Some(event)
    }
}
