//! Mushroom Kingdom's `?` blocks (grinishie1.c, items on): two bricks carry
//! a `?` from the start, each breaks into an item (it_8026F7C8) and starts
//! its wait, and the wait's end turns another brick; when both waits end
//! close together every brick turns. A `?` block is a Ground of its own
//! (map 2) that only draws over its brick.
use super::{procs::Inishie1Stage, rand_range, BrickPhase, Parameters};
use gekko_math::{msl::fctiwz, HsdRng};

/// The `?` block's map (setupStageCallbacks(2)).
pub const HATENA_MAP: u8 = 2;
/// The scheduler's key for brick `b`'s `?` Ground is `HATENA_KEY_BASE + b`:
/// several map-2 Grounds live at once, and the other maps keep their ids.
pub const HATENA_KEY_BASE: u8 = 16;
/// HATENA_APPEAR_CHECKLOOP's tries before its assert.
const APPEAR_TRIES: i32 = 20;
/// Bricks (`BLOCK_COUNT`).
const BLOCKS: usize = 19;

/// A brick's `?` status (`grInishie1_Block.status`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hatena {
    /// 1: the first wait's (xC6).
    First = 1,
    /// 2: the second wait's (xC8).
    Second = 2,
    /// 3: the all-`?` event's.
    All = 3,
}
impl Hatena {
    pub fn from_status(status: i16) -> Option<Self> {
        match status {
            1 => Some(Self::First),
            2 => Some(Self::Second),
            3 => Some(Self::All),
            _ => None,
        }
    }
}

/// A `?` block's Ground (`grInishie1_GroundVars2`, grInishie1_801FAC04):
/// what it draws. Nothing else reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HatenaObject {
    /// xC8: blink ticks left.
    pub flicker: i32,
    /// xCC: shown steadily (set over a brick that is not back yet, and for
    /// the two blocks a match starts with).
    pub steady: bool,
    /// The model's JOBJ_HIDDEN.
    pub hidden: bool,
    /// Its GObj's address is the scene's to reuse (made during the match,
    /// not before the boundary or during the stage's creation).
    pub addressed: bool,
}
impl HatenaObject {
    /// grInishie1_801FAC58 (0x801FAC58): the blink; the model then follows
    /// its brick's bone.
    pub fn step(&mut self) {
        if !self.steady && self.flicker > 0 {
            self.hidden = self.flicker & 1 == 0;
            self.flicker -= 1;
        }
    }
}

/// What the `?` blocks do to the scene.
pub trait HatenaEngine {
    fn rng(&mut self) -> &mut HsdRng;
    /// setupStageCallbacks(2): the `?` Ground over `block`'s brick. True
    /// when the scene tracks the new GObj's address.
    fn create(&mut self, block: usize) -> bool;
    /// Ground_801C4A08 on `block`'s `?` Ground; `rank` is its place among
    /// the live tracked ones, oldest first (`None`: not tracked).
    fn remove(&mut self, block: usize, rank: Option<usize>);
    /// grInishie1_801FBA34 (0x801FBA34): it_8026F7C8 five above the brick.
    fn drop_item(&mut self, block: usize);
}

/// `(int)` of a float parameter (the C passes floats to int arguments).
fn int(value: f32) -> i32 {
    fctiwz(value)
}

impl Inishie1Stage {
    /// grInishie1_801FBAA0 (0x801FBAA0): `block`'s `?` Ground.
    fn create_hatena<E: HatenaEngine>(&mut self, block: usize, engine: &mut E) {
        assert!(
            self.hatena_objects[block].is_none(),
            "grinishie1.c:926: !mapgp->u.map.block[ix].hatena_gobj"
        );
        // grInishie1_801FAC04, then xCC over a brick not back yet.
        self.hatena_objects[block] = Some(HatenaObject {
            flicker: i32::from(self.parameters.hatena_flicker),
            steady: matches!(
                self.bricks[block].phase,
                BrickPhase::Broken | BrickPhase::Returning
            ),
            hidden: false,
            addressed: false,
        });
        self.hatena_order[usize::from(self.hatena_count)] = block as u8;
        self.hatena_count += 1;
        let addressed = engine.create(block);
        self.hatena_objects[block].as_mut().unwrap().addressed = addressed;
    }

    /// grInishie1_801FBC4C (0x801FBC4C): `block`'s `?` Ground goes.
    fn remove_hatena<E: HatenaEngine>(&mut self, block: usize, engine: &mut E) {
        let count = usize::from(self.hatena_count);
        let place = self.hatena_order[..count]
            .iter()
            .position(|&b| usize::from(b) == block)
            .expect("a live `?` Ground");
        let addressed = |b: &u8| self.hatena_objects[usize::from(*b)].is_some_and(|o| o.addressed);
        let rank = addressed(&(block as u8))
            .then(|| self.hatena_order[..place].iter().filter(|b| addressed(b)).count());
        self.hatena_order.copy_within(place + 1..count, place);
        self.hatena_count -= 1;
        self.hatena_objects[block] = None;
        engine.remove(block, rank);
    }

    /// The live `?` Grounds' bricks, oldest first.
    pub fn hatena_blocks(&self) -> &[u8] {
        &self.hatena_order[..usize::from(self.hatena_count)]
    }

    /// Restore a `?` Ground a boundary holds (in creation order).
    pub fn restore_hatena(&mut self, block: usize, object: HatenaObject) {
        self.hatena_objects[block] = Some(object);
        self.hatena_order[usize::from(self.hatena_count)] = block as u8;
        self.hatena_count += 1;
    }

    /// grInishie1_801FAD84's tail (grinishie1.c:465-492): with items on
    /// outside Sudden Death and any common item switched on, two different
    /// bricks start as the two waits' `?` blocks, shown steadily.
    pub fn place_initial_hatena<E: HatenaEngine>(&mut self, engine: &mut E) {
        let first = engine.rng().randi(BLOCKS as i32) as usize;
        let second = loop {
            let second = engine.rng().randi(BLOCKS as i32) as usize;
            if second != first {
                break second;
            }
        };
        for (block, status) in [(first, Hatena::First), (second, Hatena::Second)] {
            self.bricks[block].hatena = status as i16;
            self.create_hatena(block, engine);
            self.hatena_objects[block].as_mut().unwrap().steady = true;
        }
    }

    /// grInishie1_801FB3F0's prologue (0x801FB3F0, grinishie1.c:617-640):
    /// the all-`?` event, else each wait's end turns a brick.
    pub fn step_hatena<E: HatenaEngine>(&mut self, engine: &mut E) {
        let p = self.parameters;
        if self.all_hatena_cooldown > 0 {
            self.all_hatena_cooldown -= 1;
        }
        let [first, second] = self.hatena_waits;
        let window = p.all_hatena_window;
        if self.all_hatena_cooldown == 0
            && ((first == 1 && second > 0 && second < window)
                || (second == 1 && first > 0 && first < window))
        {
            self.hatena_waits = [0; 2];
            for block in 0..BLOCKS {
                self.bricks[block].hatena = Hatena::All as i16;
                self.create_hatena(block, engine);
                self.bricks[block].respawn = 0;
            }
            return;
        }
        for (wait, status, line) in [(0, Hatena::First, 0x2D0), (1, Hatena::Second, 0x2EB)] {
            self.appear(wait, status, line, engine);
        }
    }

    /// HATENA_APPEAR_CHECKLOOP: the wait counts down; at zero a random
    /// plain brick other than the last one turns.
    fn appear<E: HatenaEngine>(&mut self, wait: usize, status: Hatena, line: u32, engine: &mut E) {
        if self.hatena_waits[wait] <= 0 {
            return;
        }
        self.hatena_waits[wait] -= 1;
        if self.hatena_waits[wait] != 0 {
            return;
        }
        let mut tries = 0;
        let mut block;
        loop {
            block = engine.rng().randi(BLOCKS as i32);
            if block != i32::from(self.hatena_last[wait]) && self.bricks[block as usize].hatena == 0 {
                break;
            }
            tries += 1;
            if tries >= APPEAR_TRIES {
                break;
            }
        }
        assert!(tries != APPEAR_TRIES, "grinishie1.c:{line}: no brick for a `?` block");
        self.hatena_last[wait] = block as i16;
        let block = block as usize;
        self.bricks[block].hatena = status as i16;
        self.create_hatena(block, engine);
        self.bricks[block].respawn = 0;
    }

    /// grInishie1_801FB0AC (0x801FB0AC): a breaking `?` brick drops its
    /// item, loses its `?` Ground and restarts its wait (the all-`?`
    /// event's last block restarts both and the event's cooldown).
    pub fn break_hatena<E: HatenaEngine>(&mut self, block: usize, engine: &mut E) {
        let p: Parameters = self.parameters;
        let Some(status) = Hatena::from_status(self.bricks[block].hatena) else {
            assert!(
                self.hatena_objects[block].is_none(),
                "grinishie1.c:669: a plain brick with a `?` Ground"
            );
            return;
        };
        if status == Hatena::All {
            unimplemented!(
                "grinishie1.c:651-670: an all-`?` brick breaks (no exact witness: every recorded one was an item's own bump)"
            );
        }
        engine.drop_item(block);
        self.bricks[block].hatena = 0;
        self.remove_hatena(block, engine);
        let first = |rng: &mut HsdRng| {
            rand_range(int(p.hatena1_wait[1]), int(p.hatena1_wait[0]), rng) as i16
        };
        let second = |rng: &mut HsdRng| {
            rand_range(int(p.hatena2_wait[1]), int(p.hatena2_wait[0]), rng) as i16
        };
        match status {
            Hatena::First => self.hatena_waits[0] = first(engine.rng()),
            Hatena::Second => self.hatena_waits[1] = second(engine.rng()),
            Hatena::All => {
                if self.bricks.iter().all(|b| b.hatena != Hatena::All as i16) {
                    self.hatena_waits[0] = first(engine.rng());
                    self.hatena_waits[1] = second(engine.rng());
                }
                self.all_hatena_cooldown = int(p.all_hatena_cooldown);
            }
        }
    }
}
