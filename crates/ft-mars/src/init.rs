//! Marth load/reset hooks, ft/kinds/ftMars/ftmars.c.
use crate::attributes::MarsAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use ft_mars_family::{MarsFamily, Specials};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Marth {
    pub attributes: MarsAttributes,
    /// mv.ms, +222C and the retained mv+4 word of the shared specials.
    pub specials: Specials,
    /// ftMs_Init_OnDeath resets model groups 0 and 1.
    pub model_groups: [i32; 2],
}
impl Marth {
    pub fn new(attributes: MarsAttributes) -> Self {
        Self {
            attributes,
            specials: Specials::default(),
            model_groups: [0; 2],
        }
    }
}
impl MarsFamily for Marth {
    const RELEASE_EFFECTS: [u16; 2] = [0x4F2, 0x4F3];
    const CHARGE_COLOR_ANIMATION: u8 = 99;
    const COUNTER_EFFECT: u16 = 0x4F1;
    const COUNTER_SETS_HIT_DAMAGE: bool = false;
    fn attributes(&self) -> &MarsAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; ft_mars_family::SPECIAL_COUNT] =
    ft_mars_family::rows::<Marth>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Marth>();

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
        &ft_mars_family::special_moves();
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &SPECIAL_ROWS;
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        air: bool,
        a: &melee_ft::fighter::assets::FighterAssets,
    ) {
        ft_mars_family::enter_special::<Self>(f, slot, air, a);
    }

    fn accessory(
        f: &mut melee_ft::fighter::Fighter,
        a: &melee_ft::fighter::assets::FighterAssets,
        _rng: &mut gekko_math::HsdRng,
    ) {
        ft_mars_family::special_n::accessory::<Self>(f, a);
    }
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = ft_mars_family::retained_scratch_word::<Self>;
    const DEFENSE_CONTACT: Option<melee_ft::fighter::DefenseContact> =
        Some(ft_mars_family::special_lw::contact::<Self>);
    const PROCESS_DEFENSE_HIT: Option<melee_ft::fighter::DefenseHit> =
        Some(ft_mars_family::special_lw::process_hit::<Self>);
    /// No special reads fp->item_gobj; a held item stays in hand.
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const ITEM_DEFENSE_CONTACT: Option<melee_ft::fighter::ItemDefenseContact> =
        Some(ft_mars_family::special_lw::item_contact::<Self>);
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
        self.specials.side_special_boost_used =
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
        self.specials.reset();
        self.model_groups = [0; 2];
    }
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:64-67.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.specials.side_special_boost_used = false;
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
