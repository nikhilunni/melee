//! Marth load/reset hooks, ft/kinds/ftMars/ftmars.c.
use crate::attributes::MarsAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Marth {
    pub attributes: MarsAttributes,
    pub special_n: crate::special_n::SpecialN,
    pub special_side: crate::special_s::SpecialSide,
    pub special_lw: crate::special_lw::SpecialLw,
    pub special_hi: crate::special_hi::SpecialHi,
    /// Fighter +222C, ftmarsspecials.c:56-60: once-per-airtime vertical boost.
    pub side_special_boost_used: bool,
    /// ftMs_Init_OnDeath resets model groups 0 and 1.
    pub model_groups: [i32; 2],
    /// mv+4 in every Marth special. ftMars_MotionVars has only an x0 word
    /// (types.h), so no special writes mv+4: it keeps the word the state
    /// before the special left (`None` when the port does not model it), and
    /// a landing from a special inherits it (ftCo_Landing.c:41-50).
    pub retained_scratch_word: Option<f32>,
}
impl Marth {
    pub fn new(attributes: MarsAttributes) -> Self {
        Self {
            retained_scratch_word: None,
            attributes,
            special_hi: Default::default(),
            special_lw: Default::default(),
            special_side: Default::default(),
            special_n: Default::default(),
            side_special_boost_used: false,
            model_groups: [0; 2],
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Marth>();

/// Marth's special actions, SpecialNStart (341) through SpecialAirLwHit (372).
const SPECIALS: std::ops::RangeInclusive<u16> = 341..=372;

/// Called by each special's entry before it changes motion state: capture
/// the mv+4 word the current state leaves behind. From one special into
/// another the word is already the retained one, so it carries through.
pub fn retain_scratch_word(f: &mut melee_ft::fighter::Fighter) {
    let word = f.inherited_scratch_word();
    f.character.get_mut::<Marth>().retained_scratch_word = word;
}

/// mv+4 while a Marth special is current (see `Marth::retained_scratch_word`).
fn special_scratch_word(marth: &Marth, action: melee_ft::fighter::ActionId) -> Option<f32> {
    if !SPECIALS.contains(&action.0) {
        return None;
    }
    Some(marth.retained_scratch_word.unwrap_or_else(|| {
        unimplemented!("ftMars specials: mv+4 inherited from an unmodelled scratch word")
    }))
}

impl CharacterCallbacks for Marth {
    const KNOCKBACK_ENTER: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    const KNOCKBACK_EXIT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &crate::special_moves();
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &crate::special_rows();
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        air: bool,
        a: &melee_ft::fighter::assets::FighterAssets,
    ) {
        match slot {
            melee_ft::fighter::SpecialSlot::Neutral => crate::special_n::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Side => crate::special_s::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Up => crate::special_hi::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Down => crate::special_lw::enter(f, air, a),
        }
    }

    fn accessory(f: &mut melee_ft::fighter::Fighter, a: &melee_ft::fighter::assets::FighterAssets) {
        crate::special_n::accessory(f, a);
    }
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| special_scratch_word(state.get::<Self>(), action);
    const DEFENSE_CONTACT: Option<melee_ft::fighter::DefenseContact> =
        Some(crate::special_lw::contact);
    const PROCESS_DEFENSE_HIT: Option<melee_ft::fighter::DefenseHit> =
        Some(crate::special_lw::process_hit);
    /// No special reads fp->item_gobj; a held item stays in hand.
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const ITEM_DEFENSE_CONTACT: Option<melee_ft::fighter::ItemDefenseContact> =
        Some(crate::special_lw::item_contact);
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    fn third_jab_state(&self) -> melee_types::CommonMotionState {
        melee_types::CommonMotionState::Attack11
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Mars
    }
    fn descriptor() -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_mars_attributes(data)?))
    }
    /// Marth +222C: side-special boost already spent (ftmars.c reset state).
    fn restore_saved(&mut self, raw: &[u8]) {
        self.side_special_boost_used =
            u32::from_be_bytes(raw[0x222C..0x2230].try_into().unwrap()) != 0;
    }
    /// ftMs_Init_OnLoad (801364AC): PUSH_ATTRS only; no item registrations
    /// or walljump flag. Four specials are present in ftdata.c's callback tables.
    /// OnLoadForRoy (80136474) uses this same attribute type, without scaling.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftMs_Init_OnDeath (80136258): two model groups and Fighter +222C.
    fn on_reset(&mut self) {
        self.special_hi = Default::default();
        self.special_lw = Default::default();
        self.special_side = Default::default();
        self.special_n = Default::default();
        self.model_groups = [0; 2];
        self.side_special_boost_used = false;
    }
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:64-67.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.side_special_boost_used = false;
    }
    /// ftCo_800923B4 (800923B4), ftCo_800939B4 (800939B4),
    /// ftCo_Guard.c:342-346,924-928: select the sword model after shield setup.
    /// Shield joint/neutral pose still come from PlMs.dat's shared descriptors.
    fn guard_variant(&self, commands: &mut melee_ft::fighter::commands::CommandState) {
        use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
        const SWORD_MODEL_GROUP: i32 = 1;
        const SHIELDED_SWORD_MODEL: i32 = 1;
        const SHIELD_SOUND: u32 = 190115;
        commands
            .model_selections
            .insert(SWORD_MODEL_GROUP, SHIELDED_SWORD_MODEL);
        commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: SHIELD_SOUND,
            volume: 127,
            pan: 64,
        });
    }
}

/// ftMs_Init_* strings, ftData_Table_Unk0[18] (327 rows), and
/// PlCo ftPartsTable[18]: 90 joints mapped to the 54 semantic parts.
pub const DESCRIPTOR: melee_ft::fighter::assets::CharacterDescriptor =
    melee_ft::fighter::assets::CharacterDescriptor {
        kind: FighterKind::Mars,
        common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mars),
        data_file: "PlMs.dat",
        data_symbol: "ftDataMars",
        animation_file: "PlMsAJ.dat",
        animation_count: 327,
        part_count: 54,
        part_animation_count: 3,
        // A3 combat scripts: smash, tumble, prone recovery, tech and linked throws.
        additional_part_animations: &[],
        costumes: &[
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsNr.dat",
                joint_symbol: "PlyMars5K_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsRe.dat",
                joint_symbol: "PlyMars5KRe_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsGr.dat",
                joint_symbol: "PlyMars5KGr_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsBk.dat",
                joint_symbol: "PlyMars5KBk_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsWh.dat",
                joint_symbol: "PlyMars5KWh_Share_joint",
            },
        ],
    };
