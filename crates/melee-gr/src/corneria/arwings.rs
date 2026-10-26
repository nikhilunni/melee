//! Corneria's Arwings and Wolfens (grcorneria.c:263-461, 744-1532).
//!
//! In a versus match only slot 0 is used (grCn_804D69B0 is set by a 1P
//! stage alone). A pass is three maps, created in this order:
//!
//! - the path (map 1, animation grCn_803E2030[type]), at the Great Fox's
//!   height; it ends the pass when its first AObj ends;
//! - the ship (map 2 for an Arwing, map 10 for a Wolfen), turned a quarter
//!   about y. A near pass (types 1..9) stands at the path's bone 4 seen
//!   from the side (its z for x, offset by grCn's lbl_803E2068 and the
//!   Great Fox's drift, never below y 100), maneuvers on bone 1 and fires
//!   its cannon level (it_802E72E0); a far pass (types 10..13) is held to
//!   the path's bone 4 (lb_8000C2F8) and fires at the fighters
//!   (it_802E7654);
//! - for a near pass the platform on its wing (map 13 with collision joint
//!   0; map 16 with joint 5 for a Wolfen), placed on the ship's bone 1.
use super::Parameters;
use gekko_math::{msl::fctiwz, HsdRng};
use hsd_types::Vec3;

/// The path model and the bone the ships follow.
pub const PATH_MAP: u8 = 1;
pub const PATH_BONE: usize = 4;
/// The path's bone 1, lowered 349 every tick (grCorneria_801DE024).
pub const PATH_DROP_BONE: usize = 1;
pub const PATH_DROP: f32 = 349.0;
/// The ship's bone a near pass banks and carries the platform on
/// (grCn_803E21B0 for every group).
pub const SHIP_BONE: usize = 1;
/// The guns: an Arwing's cannon and left gun (5), its right gun (6), a
/// Wolfen's (7).
pub const LEFT_GUN_BONE: usize = 5;
pub const RIGHT_GUN_BONE: usize = 6;
pub const WOLFEN_GUN_BONE: usize = 7;
/// grCn_803E2190's collision joints for an Arwing's and a Wolfen's
/// platform.
pub const ARWING_PLATFORM_JOINT: i32 = 0;
pub const WOLFEN_PLATFORM_JOINT: i32 = 5;

/// The groups (grCn arwing_groups in a versus match).
pub const ARWING: i32 = 1;
pub const WOLFEN: i32 = 4;

/// grCn_803E2110: the ship's map per group.
pub fn ship_map(group: i32) -> u8 {
    [2, 2, 2, 2, 10][group as usize]
}
/// grCn_803E217C: the platform's map per group.
pub fn platform_map(group: i32) -> u8 {
    [13, 13, 14, 15, 16][group as usize]
}
/// grCn_803E2190: the platform's collision joint per group.
pub fn platform_joint(group: i32) -> i32 {
    [0, 0, 1, 2, 5][group as usize]
}

/// A pass's kind by its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// Types 1..9: beside the stage, a platform on the wing.
    Near,
    /// Types 10..13: in the distance, firing at the fighters.
    Far,
}

pub fn pass(kind: i32) -> Pass {
    match kind {
        1..=9 => Pass::Near,
        10..=13 => Pass::Far,
        _ => unreachable!("grcorneria.c: Arwing pass type {kind}"),
    }
}

/// grCn_803E2030: the path's animation per type.
pub fn path_animation(kind: i32) -> usize {
    const ANIMATIONS: [usize; 14] = [usize::MAX, 0, 1, 2, 3, 4, 5, 6, 6, 6, 7, 8, 9, 10];
    ANIMATIONS[kind as usize]
}

/// lbl_803E2068: the ship's offset from the path's bone 4 per type.
pub fn ship_offset(kind: i32) -> Vec3 {
    match kind {
        7 => Vec3::new(-40.0, 10.0, 0.0),
        8 => Vec3::new(15.0, 5.0, 0.0),
        9 => Vec3::new(190.0, -10.0, 0.0),
        _ => Vec3::ZERO,
    }
}

/// grCorneria_801DDDA8 / 801DED50's placement: the path's bone 4 (`bone`,
/// world) seen from the side, with the Great Fox's drift in x; never below
/// y 100. Retail 0x801DF150..54: fadds (-z + offset), then fadds the drift.
pub fn ship_position(kind: i32, bone: Vec3, drift_x: f32) -> Vec3 {
    let offset = ship_offset(kind);
    let mut position = Vec3::new(
        drift_x + (-bone.z + offset.x),
        bone.y + offset.y,
        bone.x + offset.z,
    );
    if position.y <= 100.0 {
        position.y = 100.0;
    }
    position
}

/// The spawner (grCn_804D69A4, 804D69A8 and the slots).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spawner {
    pub wait: i32,
    pub kinds: [i32; 3],
    pub groups: [i32; 3],
    pub slot: usize,
}

impl Spawner {
    /// grCorneria_801DCCFC (0x801DCCFC): the slots clear and the first wait
    /// (grCn_RandRange(x40, x3C); GrCn.dat's are both 600, no draw).
    pub fn create(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            wait: crate::rand::range(rng, [fctiwz(p.first_wait[0]), fctiwz(p.first_wait[1])]),
            ..Self::default()
        }
    }

    /// grCorneria_801DCE1C (0x801DCE1C), the single-slot branch: while slot
    /// 0's pass lives the wait is drawn again every tick; once it is gone
    /// the wait counts down, then a pass of a new type comes. Over the sea
    /// or the coast (here and 300 to the left, `far_allowed`) the far
    /// passes may come too. `Some(slot)` creates one.
    pub fn tick(
        &mut self,
        p: &Parameters,
        live: bool,
        far_allowed: impl FnOnce() -> bool,
        rng: &mut HsdRng,
    ) -> Option<usize> {
        if live {
            self.wait = crate::rand::range(rng, [fctiwz(p.wait[0]), fctiwz(p.wait[1])]);
            return None;
        }
        self.wait -= 1;
        if self.wait > 0 {
            return None;
        }
        let types = if far_allowed() { 13 } else { 9 };
        let mut kind = self.kinds[0];
        while kind == self.kinds[0] {
            kind = rng.randi(types) + 1;
        }
        let group = if self.groups[0] == WOLFEN || rng.randf() > p.wolfen_chance {
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

/// The ship's controller (`u.corneria2` of map 2 or 10).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ship {
    pub slot: usize,
    /// xF0: ticks since creation.
    pub clock: i32,
    /// xF4: the maneuver playing (0 none, 1..4; 5 a maneuver with no
    /// animation, which never ends).
    pub maneuver: i32,
    /// xF8: ticks to the next maneuver.
    pub maneuver_wait: i32,
    /// x100: which gun an Arwing fires next (a Wolfen leaves it 0).
    pub gun: i32,
}

/// grCn_803E214C (`[6][2]`): the ship's animation for a maneuver, by group
/// kind (0 an Arwing, 1 a Wolfen); -1 plays nothing.
const MANEUVERS: [[i32; 2]; 6] = [[-1, -1], [1, 0], [2, 2], [4, 4], [5, 5], [-1, -1]];

/// 801DED50's `get_grp_off`: 0 for groups 1..3, 1 for a Wolfen.
fn group_kind(group: i32) -> usize {
    match group {
        1..=3 => 0,
        WOLFEN => 1,
        _ => unreachable!("grcorneria.c: Arwing group {group}"),
    }
}

/// A near ship's step before it is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverStep {
    /// grAnime_801C8098(gobj, 1, 7, animation, 0.0, 1.0).
    Play(usize),
    /// HSD_JObjSetRotationZ(bone 1, 0).
    Level,
    /// The maneuver plays on (or a type-5 maneuver's nothing).
    Hold,
}

/// A shot the ship fires this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shot {
    /// it_802E72E0(gobj, bone, laser, -1.0, x70): a near ship's level
    /// cannon (type 0 with its side beams, an Arwing's; 2, a Wolfen's).
    Cannon { bone: usize, laser: i32 },
    /// it_802E7654(gobj, bone, target, laser, 3, x70): a far ship's aimed
    /// laser (1 an Arwing's, 3 a Wolfen's) at the fighter in `target_slot`,
    /// 5 above its position.
    Aimed { bone: usize, laser: i32, target_slot: usize },
}

impl Ship {
    /// grCorneria_801DE8E4 / 801DF8D0's tail: Ground_ResetStarFoxArwingState,
    /// the maneuver wait and, for an Arwing, the first gun (HSD_Randi(2)).
    pub fn create(slot: usize, group: i32, p: &Parameters, rng: &mut HsdRng) -> Self {
        let mut ship = Self {
            slot,
            maneuver_wait: fctiwz(p.maneuver_wait),
            ..Self::default()
        };
        if group != WOLFEN {
            ship.gun = rng.randi(2);
        }
        ship
    }

    /// grCorneria_801DED50's maneuvers (a near ship), before it is placed:
    /// with none playing the wait runs down and then draws one of five; a
    /// maneuver ends with the first AObj from the root (`maneuver_ended`).
    pub fn maneuver(
        &mut self,
        group: i32,
        maneuver_ended: impl FnOnce() -> bool,
        p: &Parameters,
        rng: &mut HsdRng,
    ) -> ManeuverStep {
        match self.maneuver {
            0 => {
                let step = if self.maneuver_wait <= 0 {
                    self.maneuver = rng.randi(5) + 1;
                    let animation = MANEUVERS[self.maneuver as usize][group_kind(group)];
                    // grAnime_801C8098 does nothing for a negative index.
                    match usize::try_from(animation) {
                        Ok(animation) => ManeuverStep::Play(animation),
                        Err(_) => ManeuverStep::Hold,
                    }
                } else {
                    ManeuverStep::Level
                };
                self.maneuver_wait -= 1;
                step
            }
            1..=4 => {
                if maneuver_ended() {
                    self.maneuver = 0;
                    self.maneuver_wait = fctiwz(p.maneuver_wait);
                }
                ManeuverStep::Hold
            }
            _ => ManeuverStep::Hold,
        }
    }

    /// The fire window both kinds draw (`xFC`): every `x74`th tick, one
    /// time in `x78`.
    pub fn may_fire(&self, p: &Parameters, rng: &mut HsdRng) -> bool {
        if self.clock % p.fire_period != 0 {
            return false;
        }
        let draw = if p.fire_odds != 0 { rng.randi(p.fire_odds) } else { 0 };
        draw == 0
    }

    /// A near ship's shot: inside the blast zones (`inside`) and level or
    /// upside down within three degrees (its bank `bank` wrapped by the
    /// double M_PI and M_TAU).
    pub fn cannon(&self, group: i32, inside: bool, bank: f32) -> Option<Shot> {
        if !inside {
            return None;
        }
        let mut angle = bank;
        while f64::from(angle) < -core::f64::consts::PI {
            angle = (f64::from(angle) + core::f64::consts::TAU) as f32;
        }
        while f64::from(angle) > core::f64::consts::PI {
            angle = (f64::from(angle) - core::f64::consts::TAU) as f32;
        }
        let level = gekko_math::msl::fabsf(angle) < 0.05235988;
        let inverted = gekko_math::msl::fabsf(angle) > 3.0892327;
        if !(level || inverted) {
            return None;
        }
        Some(if group_kind(group) == 1 {
            Shot::Cannon {
                bone: WOLFEN_GUN_BONE,
                laser: 2,
            }
        } else {
            Shot::Cannon {
                bone: LEFT_GUN_BONE,
                laser: 0,
            }
        })
    }

    /// A far ship's shot, the window open and its position `x` between 500
    /// and 2000: a drawn fighter (the first present slot from a random one,
    /// `slot_type` being Player_GetPlayerSlotType). An Arwing alternates
    /// its guns.
    pub fn aimed(
        &mut self,
        group: i32,
        x: f32,
        open: bool,
        rng: &mut HsdRng,
        slot_type: impl Fn(usize) -> i32,
    ) -> Option<Shot> {
        if !(500.0 < x && x < 2000.0 && open) {
            return None;
        }
        let mut slot = rng.randi(4);
        let mut tries = 0;
        loop {
            slot = (slot + tries) & 3;
            if slot_type(slot as usize) != 3 {
                break;
            }
            tries += 1;
            if tries >= 4 {
                break;
            }
        }
        let target_slot = slot as usize;
        Some(if group_kind(group) == 1 {
            Shot::Aimed {
                bone: WOLFEN_GUN_BONE,
                laser: 3,
                target_slot,
            }
        } else {
            let bone = if self.gun != 0 {
                LEFT_GUN_BONE
            } else {
                RIGHT_GUN_BONE
            };
            self.gun = (self.gun + 1) & 1;
            Shot::Aimed {
                bone,
                laser: 1,
                target_slot,
            }
        })
    }
}
