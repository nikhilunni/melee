//! Shared ftMr_ specials: Mario's code, which Dr. Mario runs as well
//! (ftDr_Init_MotionStateTable points rows 343..350 at these callbacks).
//! Character differences are the family trait's hooks and attribute data.
pub mod attributes;
mod common;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use attributes::MarioAttributes;
use melee_ft::fighter::{
    assets::FighterAssets, ActionId, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};

/// The kind checks inside the shared callbacks.
pub trait MarioFamily: CharacterCallbacks {
    /// ftMr_SpecialN_ItemFireSpawn's `fp->kind == FTKIND_MARIO` branch: what
    /// the throw flag spawns at the left hand (`hand`, part bone `bone`).
    const NEUTRAL_PROJECTILE: fn(
        &mut Fighter,
        &FighterAssets,
        &mut gekko_math::HsdRng,
        hsd_types::Vec3,
        usize,
    );
    fn attributes(&self) -> &MarioAttributes;
    fn specials(&mut self) -> &mut Specials;
    fn specials_ref(&self) -> &Specials;
}

/// ftMario_FighterVars (+222C..+2240) and the specials' motion scratch.
#[derive(Clone, Debug, Default)]
pub struct Specials {
    /// Fighter +222C, x222C_vitaminCurr (the last Megavitamin colour).
    pub vitamin_current: i32,
    /// Fighter +2230, x2230_vitaminPrev.
    pub vitamin_previous: i32,
    /// Fighter +2234, x2234_tornadoCharge: an aerial Tornado already rose.
    pub tornado_charged: bool,
    /// Fighter +2238, x2238_isCapeBoost: an aerial cape already boosted.
    pub cape_boosted: bool,
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// Super Jump Punch's steering (Fighter +6BC).
    pub super_jump_punch: special_hi::SuperJumpPunch,
    /// The Tornado's motion scratch and callbacks.
    pub tornado: special_lw::Tornado,
    /// The cape and the swing's reflector.
    pub cape: special_s::Cape,
}

/// The accessory4 callback a special installed; a motion change removes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftMr_SpecialN_ItemFireSpawn: the projectile on the script's throw flag.
    NeutralProjectile,
    /// ftMr_SpecialS_CreateCape: the cape, once per swing.
    CreateCape,
}

pub(crate) fn specials<C: MarioFamily>(f: &mut Fighter) -> &mut Specials {
    f.character.get_mut::<C>().specials()
}
pub(crate) fn specials_ref<C: MarioFamily>(f: &Fighter) -> &Specials {
    f.character.get::<C>().specials_ref()
}
pub(crate) fn attributes<C: MarioFamily>(f: &Fighter) -> &MarioAttributes {
    f.character.get::<C>().attributes()
}

/// ftMr_MS_SelfCount: rows 341..350, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 10;
const FIRST_ACTION: u16 = 341;

/// ftMr_Init_MotionStateTable / ftDr_Init_MotionStateTable rows 343..350;
/// rows 341/342 are `taunt` (Mario's are never entered).
pub const fn rows<C: MarioFamily>(taunt: [MotionRow; 2]) -> [MotionRow; SPECIAL_ROW_COUNT] {
    let mut rows = [melee_ft::fighter::state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        i += 1;
    }
    rows[0] = taunt[0];
    rows[1] = taunt[1];
    place(
        &mut rows,
        common::row(
            special_n::GROUND,
            special_n::anim,
            special_n::ground_input,
            special_n::ground_physics,
            special_n::ground_collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_n::AIR,
            special_n::anim,
            special_n::air_input,
            special_n::air_physics,
            special_n::air_collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_hi::GROUND,
            special_hi::anim::<C>,
            special_hi::input::<C>,
            special_hi::ground_physics::<C>,
            special_hi::collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_hi::AIR,
            special_hi::anim::<C>,
            special_hi::input::<C>,
            special_hi::air_physics::<C>,
            special_hi::collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_lw::GROUND,
            special_lw::ground_anim::<C>,
            special_lw::input,
            special_lw::ground_physics::<C>,
            special_lw::ground_collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_lw::AIR,
            special_lw::air_anim::<C>,
            special_lw::input,
            special_lw::air_physics::<C>,
            special_lw::air_collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_s::GROUND,
            special_s::anim,
            special_s::input,
            special_s::ground_physics::<C>,
            special_s::ground_collision::<C>,
        ),
    );
    place(
        &mut rows,
        common::row(
            special_s::AIR,
            special_s::anim,
            special_s::input,
            special_s::air_physics::<C>,
            special_s::air_collision::<C>,
        ),
    );
    rows
}

const fn place(rows: &mut [MotionRow; SPECIAL_ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// The Init_MotionStateTable move IDs: the taunt rows are
/// FtMoveId_Default, the rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    [
        None,
        None,
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialDown),
        Some(M::SpecialDown),
    ]
};

/// ftData_SpecialN/S/Hi/Lw and the aerial tables: both kinds enter the
/// same ftMr_ entries.
pub fn enter_special<C: MarioFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    airborne: bool,
    assets: &FighterAssets,
) {
    match slot {
        SpecialSlot::Neutral => special_n::enter::<C>(f, airborne, assets),
        SpecialSlot::Up => special_hi::enter::<C>(f, airborne, assets),
        SpecialSlot::Down => special_lw::enter::<C>(f, airborne, assets),
        SpecialSlot::Side => special_s::enter::<C>(f, airborne, assets),
    }
}

/// Fighter_8006C80C: the special's accessory4, installed until the next
/// motion change.
pub fn accessory<C: MarioFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) {
    if !f.core.accessory4_armed {
        return;
    }
    match specials_ref::<C>(f).accessory {
        Accessory::NeutralProjectile => special_n::item_spawn::<C>(f, assets, rng),
        Accessory::CreateCape => special_s::create_cape::<C>(f, assets),
        Accessory::None => {}
    }
}

/// take_dmg_cb / death2_cb: whichever of the Tornado's and the cape's
/// callbacks the current motion installed.
pub fn damage_callback<C: MarioFamily>(f: &mut Fighter) {
    special_lw::clear_tilt::<C>(f);
    special_s::remove_cape::<C>(f);
}

impl Specials {
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    pub fn on_motion_change(&mut self) {
        self.tornado.callbacks = false;
        self.cape.damage_callbacks = false;
    }
    /// ftCo_Landing_Enter, ftCo_Landing.c:49-52 (FTKIND_MARIO and
    /// FTKIND_DRMARIO).
    pub fn on_landing(&mut self) {
        self.tornado_charged = false;
        self.cape_boosted = false;
    }
    /// ftMario_FighterVars +222C..+223B from a saved Fighter.
    pub fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.vitamin_current = word(0x222C) as i32;
        self.vitamin_previous = word(0x2230) as i32;
        self.tornado_charged = word(0x2234) != 0;
        self.cape_boosted = word(0x2238) != 0;
        if word(0x223C) != 0 {
            unimplemented!("ftMario_FighterVars: a saved cape GObj (+223C)");
        }
    }
}

/// ftMr_SpecialN_VitaminRandom (800E0D1C): a colour other than the last
/// two (HSD_Randi over the rest), remembered as the new current one.
pub fn vitamin_random<C: MarioFamily>(f: &mut Fighter, rng: &mut gekko_math::HsdRng) -> i32 {
    /// Nine Megavitamin colour pairs.
    const COLOURS: i32 = 9;
    let s = specials::<C>(f);
    let mut choices = [0; COLOURS as usize];
    let mut count = 0;
    for colour in 0..COLOURS {
        if colour != s.vitamin_current && colour != s.vitamin_previous {
            choices[count] = colour;
            count += 1;
        }
    }
    let picked = choices[rng.randi(count as i32) as usize];
    s.vitamin_previous = s.vitamin_current;
    s.vitamin_current = picked;
    picked
}
