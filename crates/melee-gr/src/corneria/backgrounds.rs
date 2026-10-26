//! The scenery scrolling past (grcorneria.c:1540-1822): three background
//! maps, each `width * map scale` long, move right by `x88` a tick. Map 8
//! makes map 9 behind it once it is far enough along, 9 makes 4 and 4
//! makes 8 again (grCorneria_801E03C8); each goes once it is past the
//! stage. Which one is under a point (grCorneria_801E0A74) or under the
//! camera's interest (grCorneria_801E08CC) picks the Arwings' passes and
//! the Great Fox's descent.
use super::{great_fox::Scenery, Parameters};
use gekko_math::fma;

/// The background maps, in the order the lookups test them.
pub const CITY_MAP: u8 = 8;
pub const COAST_MAP: u8 = 9;
pub const SEA_MAP: u8 = 4;
pub const MAPS: [u8; 3] = [CITY_MAP, COAST_MAP, SEA_MAP];

/// A background's length before the map scale.
pub fn width(map: u8) -> f32 {
    match map {
        CITY_MAP | COAST_MAP => 3200.0,
        SEA_MAP => 4800.0,
        _ => unreachable!("grcorneria.c: background map {map}"),
    }
}

/// The background each one makes behind it.
pub fn next(map: u8) -> u8 {
    match map {
        CITY_MAP => COAST_MAP,
        COAST_MAP => SEA_MAP,
        SEA_MAP => CITY_MAP,
        _ => unreachable!("grcorneria.c: background map {map}"),
    }
}

pub fn scenery(map: u8) -> Scenery {
    match map {
        CITY_MAP => Scenery::City,
        COAST_MAP => Scenery::Coast,
        SEA_MAP => Scenery::Sea,
        _ => unreachable!("grcorneria.c: background map {map}"),
    }
}

/// `width * scale / 2` as retail evaluates it next to a point: the low end
/// is fnmsubs, the high end fmadds (retail 0x801E0930 / 094C,
/// 0x801E0AD4 / 0AF4).
fn ends(map: u8, x: f32, map_scale: f32) -> (f32, f32) {
    let length = width(map) * map_scale;
    (fma::fnmsubs(length, 0.5, x), fma::fmadds(length, 0.5, x))
}

/// Whether `point` is strictly within the background at `x`.
pub fn covers(map: u8, x: f32, map_scale: f32, point: f32) -> bool {
    let (low, high) = ends(map, x, map_scale);
    point > low && point < high
}

/// grCorneria_801E03C8's placement: the new background `map` goes behind
/// its maker `maker` (at `x`) by half of each (retail 0x801E0454: fmuls,
/// fmuls, fmadds, fsubs).
pub fn placed_behind(map: u8, maker: u8, x: f32, map_scale: f32) -> f32 {
    let half = width(map) * map_scale * 0.5;
    x - fma::fmadds(width(maker) * map_scale, 0.5, half)
}

/// A live background: its GObj's x and whether it has made the next one
/// (`u.corneria.xC5`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Background {
    pub x: f32,
    pub passed: bool,
}

/// What a background's proc does after it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scroll {
    Nothing,
    /// grCorneria_801E03C8: the next background, placed at `x`.
    MakeNext { x: f32 },
    /// Ground_801C4A08: this one goes.
    Leave,
}

impl Background {
    /// grCorneria_801DFC98 / 801DFF20 / 801E01A8: the step right, then
    /// (retail 0x801DFE24 / 7C: fmadds) the next background at 1400 short of
    /// its far end, and leaving 1400 past it.
    pub fn scroll(&mut self, map: u8, p: &Parameters, map_scale: f32) -> Scroll {
        self.x += p.scroll_speed;
        let length = width(map) * map_scale;
        if !self.passed {
            if self.x >= fma::fmadds(length, 0.5, -1400.0) {
                self.passed = true;
                return Scroll::MakeNext {
                    x: placed_behind(next(map), map, self.x, map_scale),
                };
            }
        } else if self.x >= fma::fmadds(length, 0.5, 1400.0) {
            return Scroll::Leave;
        }
        Scroll::Nothing
    }
}

/// grCorneria_801E0A74 / 801E08CC over the live backgrounds (`x` of maps 8,
/// 9 and 4, in that order): the first that covers `point`. Retail asserts
/// when none does (grcorneria.c:2508, 2507).
pub fn under(backgrounds: [Option<Background>; 3], map_scale: f32, point: f32) -> Option<u8> {
    MAPS.into_iter()
        .zip(backgrounds)
        .find(|(map, b)| b.is_some_and(|b| covers(*map, b.x, map_scale, point)))
        .map(|(map, _)| map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backgrounds_follow_one_another_without_a_gap() {
        let p = crate::corneria::test_parameters();
        let scale = 0.54;
        let mut live: [Option<Background>; 3] = [Some(Background::default()), None, None];
        let mut sequence = vec![CITY_MAP];
        for _ in 0..20_000 {
            for (index, map) in MAPS.into_iter().enumerate() {
                let Some(mut b) = live[index] else { continue };
                match b.scroll(map, &p, scale) {
                    Scroll::MakeNext { x } => {
                        let next = next(map);
                        let slot = MAPS.iter().position(|&m| m == next).unwrap();
                        assert!(live[slot].is_none(), "map {next} made twice");
                        live[slot] = Some(Background { x, passed: false });
                        sequence.push(next);
                    }
                    Scroll::Leave => {
                        live[index] = None;
                        continue;
                    }
                    Scroll::Nothing => {}
                }
                live[index] = Some(b);
            }
            for point in [-300.0, 0.0, 300.0] {
                assert!(under(live, scale, point).is_some());
            }
        }
        assert_eq!(&sequence[..4], &[CITY_MAP, COAST_MAP, SEA_MAP, CITY_MAP]);
    }
}
