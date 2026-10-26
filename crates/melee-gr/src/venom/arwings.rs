//! The Arwings and Wolfens (grvenom.c:231-410, 1026-1675; the Star Fox
//! inlines of gr/inlines.h).
//!
//! The stage keeps three slots (grVe_803E5348.arwing): each a path model's
//! GObj, its pass type 1..11 and a group (grVe_803E5380: 1 an Arwing, 4 a
//! Wolfen). In a versus match only slot 0 is used (grVe_804D6A40 is set by
//! a 1P stage alone). A pass is three maps, created and run in this order:
//!
//! - the path (map 2, animation `type - 1`): ends the pass when its first
//!   AObj ends;
//! - the ship (map 3, an Arwing; map 6, a Wolfen) placed on the path's bone
//!   5: a near pass (types 1..7) carries the platform and banks every 120
//!   ticks (one of four maneuvers on bone 1), a far pass (8..11) is held
//!   to the path's bone 5 and fires;
//! - the platform (map 10 with collision joint 4; map 13 with joint 3 for a
//!   Wolfen), placed on the ship's bone 1 each tick.
use super::{sequence::Groups, Parameters};
use gekko_math::HsdRng;
use hsd_types::Vec3;

/// The path model.
pub const PATH_MAP: u8 = 2;
/// The path's bone the ship follows (grVenom_802052E0).
pub const PATH_BONE: usize = 5;
/// The ship's bone a near pass banks and carries the platform on
/// (grVe_803E56A0 for every group).
pub const SHIP_BONE: usize = 1;
/// A far Arwing's two guns and a Wolfen's.
pub const LEFT_GUN_BONE: usize = 5;
pub const RIGHT_GUN_BONE: usize = 6;
pub const WOLFEN_GUN_BONE: usize = 7;

/// The groups (grVe_803E5380's values in a versus match).
pub const ARWING: i32 = 1;
pub const WOLFEN: i32 = 4;

/// grVe_803E5530[48 + group]: the ship's map per group.
pub fn ship_map(group: i32) -> u8 {
    [3, 3, 3, 3, 6][group as usize]
}
/// grVe_803E566C: the platform's map per group.
pub fn platform_map(group: i32) -> u8 {
    [10, 10, 11, 12, 13][group as usize]
}
/// grVe_803E5680: the platform's collision joint per group.
pub fn platform_joint(group: i32) -> i32 {
    [4, 4, 4, 4, 3][group as usize]
}

/// A pass's kind by its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// Types 1..7: in front of the stage, a platform on the wing.
    Near,
    /// Types 8..11: in the distance, firing.
    Far,
}

pub fn pass(kind: i32) -> Pass {
    match kind {
        1..=7 => Pass::Near,
        8..=11 => Pass::Far,
        _ => unreachable!("grvenom.c: Arwing pass type {kind}"),
    }
}

/// grVenom_802053B0's path animation (grVe_803E5530[type]).
pub fn path_animation(kind: i32) -> usize {
    (kind - 1) as usize
}

/// grVe_RandRange (grvenom.c:217): the wait between `fmin` and `fmax`,
/// converted toward zero.
fn rand_range(rng: &mut HsdRng, fmin: f32, fmax: f32) -> i32 {
    let low = gekko_math::msl::fctiwz(fmin);
    let high = gekko_math::msl::fctiwz(fmax);
    crate::rand::range(rng, [low, high])
}

/// The spawner's state: the wait (grVe_804D6A38), the slots' types and
/// groups, and the slot the next created pass takes (grVe_804D6A34).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spawner {
    pub wait: i32,
    pub kinds: [i32; 3],
    pub groups: [i32; 3],
    pub slot: usize,
}

impl Spawner {
    /// grVenom_80203B18's `inlineA0`: the first wait.
    pub fn create(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            wait: rand_range(rng, parameters.first_wait[0], parameters.first_wait[1]),
            ..Self::default()
        }
    }

    /// grVenom_8020362C (0x8020362C), the single-slot branch: while slot
    /// 0's pass lives the wait is drawn again every tick; once it is gone
    /// the wait counts down, then a pass of a new type comes unless the
    /// flight forbids it. `Some(slot)` creates one.
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        groups: Groups,
        live: bool,
        rng: &mut HsdRng,
    ) -> Option<usize> {
        if live {
            self.wait = rand_range(rng, parameters.wait[0], parameters.wait[1]);
            return None;
        }
        self.wait -= 1;
        if self.wait > 0 {
            return None;
        }
        let types = match (groups.tunnel, groups.near_only) {
            (false, false) => 11,
            (false, true) => 4,
            // grvenom.c:265-277: a tunnel flag returns without a pass.
            (true, _) => return None,
        };
        let mut kind = self.kinds[0];
        while kind == self.kinds[0] {
            kind = rng.randi(types) + 1;
        }
        let group = if self.groups[0] == WOLFEN || rng.randf() > parameters.wolfen_chance {
            ARWING
        } else {
            WOLFEN
        };
        self.slot = 0;
        self.kinds[0] = kind;
        self.groups[0] = group;
        Some(0)
    }
}

/// The ship's controller (`u.venom` of map 3 or 6).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ship {
    pub slot: usize,
    /// xF0: ticks since creation.
    pub clock: i32,
    /// xF4: the maneuver playing (0 none, 1..4).
    pub maneuver: i32,
    /// xF8: ticks to the next maneuver.
    pub maneuver_wait: i32,
    /// x100: which gun fires next (an Arwing's; a Wolfen leaves it 0).
    pub gun: i32,
}

/// grVe_803E5644 (`anim_args[5][2]`, +0x2FC): the ship's animation for a
/// maneuver, by group kind (0 an Arwing, 1 a Wolfen).
const MANEUVERS: [i32; 10] = [-1, -1, 1, 0, 2, 2, 4, 4, 5, 5];

/// grVenom_80205F30's `fire_kind`: 0 for groups 1..3, 1 for a Wolfen.
fn fire_kind(group: i32) -> i32 {
    match group {
        1..=3 => 0,
        WOLFEN => 1,
        _ => -1,
    }
}

/// A near ship's step before it is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverStep {
    /// grAnime_801C8098(gobj, 1, 7, animation, 0.0, 1.0).
    Play(usize),
    /// HSD_JObjSetRotationZ(bone 1, 0).
    Level,
    /// The maneuver plays on.
    Hold,
}

/// A far ship's shot (it_802E7654).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// The fighter's slot it aims at (Player_LoadPlayerCoords).
    pub target_slot: usize,
    pub bone: usize,
    /// it_802E7654's type: 1 an Arwing's laser, 3 a Wolfen's.
    pub laser: i32,
}

impl Ship {
    /// grVenom_80205AD4 / 80206874's tail: Ground_ResetStarFoxArwingState,
    /// the maneuver wait and, for an Arwing, the first gun (HSD_Randi(2)).
    pub fn create(slot: usize, group: i32, parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let mut ship = Self {
            slot,
            clock: 0,
            maneuver: 0,
            maneuver_wait: gekko_math::msl::fctiwz(parameters.maneuver_wait),
            gun: 0,
        };
        if group != WOLFEN {
            ship.gun = rng.randi(2);
        }
        ship
    }

    /// grVenom_80205F30's near branch, before the ship is placed: with no
    /// maneuver playing the wait runs down and then draws one; a maneuver
    /// ends with the first AObj from the root (`maneuver_ended`).
    pub fn maneuver(
        &mut self,
        group: i32,
        maneuver_ended: bool,
        parameters: &Parameters,
        rng: &mut HsdRng,
    ) -> ManeuverStep {
        match self.maneuver {
            0 => {
                let step = if self.maneuver_wait <= 0 {
                    self.maneuver = rng.randi(4) + 1;
                    let index = 2 * self.maneuver + fire_kind(group);
                    let animation = MANEUVERS[index as usize];
                    assert!(animation >= 0, "grVe_803E5644[{index}]: no maneuver");
                    ManeuverStep::Play(animation as usize)
                } else {
                    ManeuverStep::Level
                };
                self.maneuver_wait -= 1;
                step
            }
            1..=4 => {
                if maneuver_ended {
                    self.maneuver = 0;
                    self.maneuver_wait = gekko_math::msl::fctiwz(parameters.maneuver_wait);
                }
                ManeuverStep::Hold
            }
            _ => ManeuverStep::Hold,
        }
    }

    /// grVenom_80205F30's far branch (0x80205F30): every eighth tick a
    /// one-in-eight draw opens the pass's firing window; inside it, with
    /// the path's bone 5 between z -100 and -2000, a drawn fighter on the
    /// same side is shot at. `slot_type(slot)` is Player_GetPlayerSlotType.
    /// The clock advances after.
    pub fn fire(
        &mut self,
        kind: i32,
        group: i32,
        path: Vec3,
        rng: &mut HsdRng,
        slot_type: impl Fn(usize) -> i32,
        player_position: impl Fn(usize) -> Vec3,
    ) -> Option<(Shot, Vec3)> {
        let mut open = false;
        if self.clock & 7 == 0 && rng.randi(8) == 0 {
            let window = match kind {
                8 => (0x3C, 0xE6),
                9 => (0x32, 0xE6),
                10 => (0x5A, 0x104),
                11 => (0x46, 0xF0),
                _ => unreachable!("grvenom.c: far pass type {kind}"),
            };
            open = self.clock > window.0 && self.clock < window.1;
        }
        let mut shot = None;
        if -100.0 > path.z && path.z > -2000.0 && open {
            let mut slot = rng.randi(4);
            let mut retries = 0;
            loop {
                slot = (slot + retries) & 3;
                if slot_type(slot as usize) != 3 {
                    break;
                }
                retries += 1;
                if retries >= 4 {
                    break;
                }
            }
            let mut target = player_position(slot as usize);
            if (target.x > 0.0 && path.x > 0.0) || (target.x < 0.0 && path.x < 0.0) {
                target.y += 5.0;
                let (bone, laser) = if fire_kind(group) == 1 {
                    (WOLFEN_GUN_BONE, 3)
                } else {
                    let bone = if self.gun != 0 {
                        LEFT_GUN_BONE
                    } else {
                        RIGHT_GUN_BONE
                    };
                    self.gun = (self.gun + 1) & 1;
                    (bone, 1)
                };
                shot = Some((
                    Shot {
                        target_slot: slot as usize,
                        bone,
                        laser,
                    },
                    target,
                ));
            }
        }
        self.clock += 1;
        shot
    }

    /// The near branch's clock: it advances after the ship is placed.
    pub fn advance(&mut self) {
        self.clock += 1;
    }
}

/// The platform's controller (`u.arwing` of maps 10..13): the ship's bone
/// 1 position and bank (xE0, xDC), written by the ship each tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Platform {
    pub slot: usize,
    pub position: Vec3,
    pub bank: f32,
}

impl Platform {
    /// Ground_UpdateStarFoxArwingVisibility (inlines.h, depth 50): near the
    /// stage's plane the platform is drawn on it while banked less than 60
    /// degrees; its translation is the (flattened) position either way.
    /// Returns the translation and whether it is hidden.
    pub fn place(&mut self) -> (Vec3, bool) {
        let depth = gekko_math::msl::fabsf(self.position.z);
        let hidden = if depth < 50.0 {
            self.position.z = 0.0;
            // M_PI and M_TAU are double constants: the comparisons and
            // steps promote the float.
            while f64::from(self.bank) < -core::f64::consts::PI {
                self.bank = (f64::from(self.bank) + core::f64::consts::TAU) as f32;
            }
            while f64::from(self.bank) > core::f64::consts::PI {
                self.bank = (f64::from(self.bank) - core::f64::consts::TAU) as f32;
            }
            gekko_math::msl::fabsf(self.bank) >= core::f32::consts::FRAC_PI_3
        } else {
            true
        };
        (self.position, hidden)
    }
}
