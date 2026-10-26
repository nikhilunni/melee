//! Poké Floats (`gr/grpura.c`, GrPu.dat, StKind 0x17). Map 4 is the
//! sequence: its animation carries 26 bones, each the place of one floating
//! Pokémon, with the stage's collision joints on them; every Pokémon is its
//! own Ground (maps 2, 3 and 5..26, map 13 three times) whose root copies
//! its bone each tick. The first 25 bones are camera subjects while shown,
//! and 41 collision vertices follow joints of the animated Pokémon (maps 2,
//! 8, 12 and 25). Map 1 is the sky, whose colour fades between four.
pub mod procs;

use gekko_math::HsdRng;

/// grPu_803E6AA0: the sky's colours.
pub const SKY_COLORS: [[u8; 3]; 4] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0x50],
    [0x1E, 0x46, 0x00],
    [0x87, 0x87, 0x00],
];
/// stageGObj1_GObjProc's fade length (0xE10 ticks).
pub const SKY_FADE: i16 = 3600;

/// grPura_GroundVars on map 1: the sky fading from one colour to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sky {
    /// xC4 and xC6: the colours faded from and to.
    pub from: i16,
    pub to: i16,
    /// xC8: ticks into the fade.
    pub tick: i16,
    /// The colour last set (Ground_801C205C, Camera_SetBackgroundColor):
    /// display only.
    pub color: [u8; 3],
}

impl Sky {
    /// stageGObj1_OnInit (0x80212024): a colour, then a different one.
    pub fn new(rng: &mut HsdRng) -> Self {
        let from = rng.randi(4) as i16;
        let mut sky = Self {
            from,
            to: from,
            tick: 0,
            color: SKY_COLORS[from as usize],
        };
        sky.draw_next(rng);
        sky
    }

    /// `do { xC6 = HSD_Randi(4); } while (xC4 == xC6)` (sth, then cmpw).
    fn draw_next(&mut self, rng: &mut HsdRng) {
        loop {
            self.to = rng.randi(4) as i16;
            if self.to != self.from {
                break;
            }
        }
    }

    /// stageGObj1_GObjProc (0x802120E0): the fade's next colour, or at its
    /// end a new target drawn (the colour stays).
    pub fn step(&mut self, rng: &mut HsdRng) {
        if self.tick < SKY_FADE {
            self.color = self.faded(self.tick);
            self.tick += 1;
            return;
        }
        self.from = self.to;
        self.draw_next(rng);
        self.tick = 0;
    }

    /// `t * (to - from) + from` per channel, `t = tick / 3600` (0x80212170
    /// fdivs, 0x802121A4 fmadds, fctiwz, stb).
    fn faded(&self, tick: i16) -> [u8; 3] {
        let from = SKY_COLORS[self.from as usize];
        let to = SKY_COLORS[self.to as usize];
        let t = f32::from(tick) / f32::from(SKY_FADE);
        std::array::from_fn(|i| {
            let diff = f32::from(i16::from(to[i]) - i16::from(from[i]));
            let value = gekko_math::fma::fmadds(t, diff, f32::from(from[i]));
            gekko_math::msl::fctiwz(value) as u8
        })
    }
}

/// grPu_803E6AB0: the Pokémon, in creation order: the map each is, and the
/// sequence's collision joint (Ground_801C33C0(4, joint)) whose bone it
/// follows. Every scale factor is 1.0.
pub const FLOATERS: [(u8, i32); 26] = [
    (2, 0),
    (3, 1),
    (5, 2),
    (6, 3),
    (7, 4),
    (8, 5),
    (9, 6),
    (10, 7),
    (11, 8),
    (12, 9),
    (13, 10),
    (13, 11),
    (13, 12),
    (14, 13),
    (15, 14),
    (16, 15),
    (17, 16),
    (18, 17),
    (19, 18),
    (20, 19),
    (21, 20),
    (22, 21),
    (23, 22),
    (24, 23),
    (25, 24),
    (26, 25),
];

/// grPu_803E6C0C: collision vertices placed at a Pokémon's joint every tick
/// (grPura_80213030): vertex, map, bone.
pub const VERTICES: [(i32, u8, usize); 41] = [
    (20, 2, 21),
    (21, 2, 22),
    (22, 2, 23),
    (23, 2, 24),
    (24, 2, 25),
    (25, 2, 26),
    (26, 2, 27),
    (39, 2, 28),
    (284, 12, 21),
    (285, 12, 23),
    (267, 12, 24),
    (261, 12, 25),
    (262, 12, 26),
    (275, 12, 27),
    (276, 12, 28),
    (277, 12, 29),
    (269, 12, 30),
    (270, 12, 31),
    (272, 12, 32),
    (286, 12, 33),
    (180, 8, 23),
    (181, 8, 51),
    (182, 8, 27),
    (183, 8, 30),
    (184, 8, 31),
    (513, 25, 10),
    (514, 25, 13),
    (520, 25, 16),
    (521, 25, 19),
    (522, 25, 22),
    (496, 25, 25),
    (497, 25, 28),
    (500, 25, 31),
    (523, 25, 32),
    (524, 25, 29),
    (527, 25, 26),
    (528, 25, 23),
    (525, 25, 20),
    (526, 25, 17),
    (494, 25, 14),
    (495, 25, 11),
];

/// grPura_80213030's mpJointUpdateBounding calls, in order.
pub const BOUNDED_JOINTS: [i32; 4] = [0, 9, 0x18, 5];
/// stageGObj4_GObjProc's mpLib_80055E24 joint.
pub const REFRESHED_JOINT: i32 = 0x18;
/// grPura_80212CD4: at most 25 camera subjects.
pub const SUBJECTS: usize = 25;
/// grPura_80212CD4's subject extents: v (30, -30), h (-30, 30).
pub const SUBJECT_EXTENT: f32 = 30.0;

/// The stage's controllers: map 1's sky and map 4's camera subjects
/// (grPura_GroundVars3 x128).
#[derive(Clone)]
pub struct Pura {
    pub sky: Sky,
    /// x128[i], on the sequence's bone `subject_bones[i]` (xC4[i]).
    pub subjects: [melee_cm::Subject; SUBJECTS],
    pub subject_bones: [usize; SUBJECTS],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_draws_a_different_second_colour_and_fades_for_3600_ticks() {
        let mut rng = HsdRng::new(1);
        let mut sky = Sky::new(&mut rng);
        assert_ne!(sky.from, sky.to);
        let first = sky.to;
        for _ in 0..SKY_FADE {
            sky.step(&mut rng);
        }
        assert_eq!(sky.tick, SKY_FADE);
        sky.step(&mut rng);
        assert_eq!((sky.from, sky.tick), (first, 0));
        assert_ne!(sky.to, sky.from);
    }
}
