//! Mario load/reset hooks, ft/kinds/ftMario/ftmario.c.
use ft_mario_family::{
    attributes::{read_mario_attributes, MarioAttributes},
    MarioFamily, Specials,
};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets, Result as AssetResult},
    Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

/// ftMr_Init_OnDeath resets the Megavitamin colours to this (no colour).
const VITAMIN_RESET: i32 = 9;

#[derive(Clone, Debug)]
pub struct Mario {
    pub attributes: MarioAttributes,
    /// ftMr_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// ftMario_FighterVars and the specials' scratch.
    pub specials: Specials,
}
impl Mario {
    pub fn new(attributes: MarioAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            specials: Specials {
                vitamin_current: VITAMIN_RESET,
                vitamin_previous: VITAMIN_RESET,
                ..Specials::default()
            },
        }
    }
}

/// ftMr_Init_MotionStateTable: AppealSR/AppealSL (341/342) are Dr. Mario's
/// taunt rows with no callbacks; Mario never enters them.
static SPECIAL_ROWS: [MotionRow; ft_mario_family::SPECIAL_ROW_COUNT] =
    ft_mario_family::rows::<Mario>([melee_ft::fighter::state::unimplemented_row(); 2]);
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Mario>();

impl MarioFamily for Mario {
    const NEUTRAL_PROJECTILE: fn(
        &mut Fighter,
        &FighterAssets,
        &mut gekko_math::HsdRng,
        hsd_types::Vec3,
        usize,
    ) = ft_mario_family::special_n::spawn_fireball;
    fn attributes(&self) -> &MarioAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

impl CharacterCallbacks for Mario {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> AssetResult<()> =
        Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_mario_family::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Mario] and the aerial tables.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        ft_mario_family::enter_special::<Self>(f, slot, airborne, assets);
    }
    /// ftCommon_8007DB58: take_dmg_cb, the Tornado's updateRot or the
    /// cape's ftMr_Init_OnTakeDamage (one installed at a time).
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(ft_mario_family::damage_callback::<Self>);
    /// ftCo_800D331C: death2_cb, the same pair.
    const DEATH: Option<fn(&mut Fighter)> = Some(ft_mario_family::damage_callback::<Self>);
    fn on_motion_change(&mut self) {
        self.specials.on_motion_change();
    }
    /// itMarioCape_Logic41_Destroyed: the cape resets its owner.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) =
        ft_mario_family::special_s::article_destroyed::<Self>;
    /// The cape's reflector (ftColl_CreateReflectHit, no hit callback).
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(ft_mario_family::special_s::reflector_contact::<Self>);
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(ft_mario_family::special_s::reflect_hit);
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        ft_mario_family::accessory::<Self>(f, assets, rng);
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Mario
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_mario_attributes(data)?))
    }
    /// ftMario_FighterVars +222C..+223B. The cape GObj (+223C) and +2240
    /// are not modelled; a save with either set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.specials.restore_saved(raw);
        if u32::from_be_bytes(raw[0x2240..0x2244].try_into().unwrap()) != 0 {
            unimplemented!("ftMario_FighterVars: a saved +2240");
        }
    }
    /// ftMr_Init_OnLoad (800E0960): can_walljump and PUSH_ATTRS. The fire
    /// and cape item registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftMr_Init_OnDeath (800E08CC): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset.
    fn on_reset(&mut self) {
        self.model_group = 0;
        let s = &mut self.specials;
        s.vitamin_current = VITAMIN_RESET;
        s.vitamin_previous = VITAMIN_RESET;
        s.tornado_charged = false;
        s.cape_boosted = false;
    }
    /// ftCo_Landing_Enter, ftCo_Landing.c:49-52.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.specials.on_landing();
    }
}

/// ftMr_Init_* strings (ftmariostrings.c); ftData_Table_Unk0[0] has 303
/// animation rows. PlCo ftPartsTable maps Mario's joints to 54 parts;
/// ftData.x1C holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Mario,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mario),
    data_file: "PlMr.dat",
    data_symbol: "ftDataMario",
    animation_file: "PlMrAJ.dat",
    animation_count: 303,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlMrNr.dat",
            joint_symbol: "PlyMario5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrYe.dat",
            joint_symbol: "PlyMario5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrBk.dat",
            joint_symbol: "PlyMario5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrBu.dat",
            joint_symbol: "PlyMario5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrGr.dat",
            joint_symbol: "PlyMario5KGr_Share_joint",
        },
    ],
};
