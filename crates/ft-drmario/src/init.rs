//! Dr. Mario load/reset hooks, ft/kinds/ftDrMario/ftdrmario.c.
use ft_mario_family::{
    attributes::{read_attributes, MarioAttributes},
    MarioFamily, Specials,
};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets, Result as AssetResult},
    Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct DrMario {
    pub attributes: MarioAttributes,
    /// ftDr_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// ftMario_FighterVars and the specials' scratch (ftDr_Init_MotionStateTable
    /// shares rows 343..350 with Mario).
    pub specials: Specials,
    /// Fighter +2240: the side taunt's Megavitamin and its damage callbacks.
    pub taunt: crate::megavitamin::TauntPill,
}
impl DrMario {
    /// The Megavitamin colours start as whatever HSD_ObjAlloc left in the
    /// Fighter block (fighter.c:854-858); a saved Fighter restores them.
    pub fn new(attributes: MarioAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            specials: Specials::default(),
            taunt: Default::default(),
        }
    }
}

/// ftDr_Init_MotionStateTable: the side taunt (341/342), then Mario's rows.
static SPECIAL_ROWS: [MotionRow; ft_mario_family::SPECIAL_ROW_COUNT] =
    ft_mario_family::rows::<DrMario>(crate::megavitamin::TAUNT_ROWS);
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<DrMario>();

impl MarioFamily for DrMario {
    /// ftMr_SpecialN_ItemFireSpawn's else branch: itDrMarioPill_Spawn.
    const NEUTRAL_PROJECTILE: fn(
        &mut Fighter,
        &FighterAssets,
        &mut gekko_math::HsdRng,
        hsd_types::Vec3,
        usize,
    ) = crate::megavitamin::throw;
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

impl CharacterCallbacks for DrMario {
    const MOTION_FLAGS: &'static [u32] = &MOTION_FLAGS;
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28's FTKIND_DRMARIO arm: ftDr_Init_80149910.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> AssetResult<()> =
        crate::megavitamin::enter_taunt;
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    /// No row of ftDr_Init_MotionStateTable reads fp->item_gobj
    /// (ftmariospecial*.c, ftdrmarioappeals.c): a held light item stays in
    /// hand through the Megavitamin throw, the sheet, the punch, the tornado
    /// and the side taunt, whose pill is not held (itDrMarioPill_Appeal_Spawn
    /// follows the hand through its own accessory callback). Retail
    /// witnesses, Peach's turnip in hand: `slp_ps_drmario_peach_fzw_t5600`
    /// (343, 344) and `drmario_held_turnip_*_ps_peach` (341, 345..350; 342
    /// is never entered: ftCo_800DEAE8 picks 341 facing either way).
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_mario_family::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[DrMario]: Mario's entries.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        ft_mario_family::enter_special::<Self>(f, slot, airborne, assets);
    }
    /// take_dmg_cb: the Tornado's updateRot, the sheet's
    /// ftMr_Init_OnTakeDamage or the taunt's ftDr_Init_80149540.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(damage_callback);
    /// death2_cb: the same three.
    const DEATH: Option<fn(&mut Fighter)> = Some(damage_callback);
    fn on_motion_change(&mut self) {
        self.specials.on_motion_change();
        self.taunt.damage_callbacks = false;
    }
    /// itMarioCape_Logic41_Destroyed: the sheet resets its owner; the taunt
    /// pill lets go of him (ftDr_Init_801498A0).
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = |f, kind| {
        ft_mario_family::special_s::article_destroyed::<Self>(f, kind);
        crate::megavitamin::released(f, kind);
    };
    /// The sheet's reflector (ftColl_CreateReflectHit, no hit callback).
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(ft_mario_family::special_s::reflector_contact::<Self>);
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(ft_mario_family::special_s::reflect_hit);
    /// Fighter_8006C80C: the special's accessory4.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        ft_mario_family::accessory::<Self>(f, assets, rng);
    }

    fn kind(&self) -> FighterKind {
        FighterKind::DrMario
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_attributes(data, DESCRIPTOR.data_symbol)?))
    }
    /// ftMario_FighterVars +222C..+2243. A saved sheet GObj (+223C) or
    /// taunt pill (+2240) fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.specials.restore_saved(raw);
        if u32::from_be_bytes(raw[0x2240..0x2244].try_into().unwrap()) != 0 {
            unimplemented!("ftMario_FighterVars: a saved taunt Megavitamin (+2240)");
        }
    }
    /// +222C and +2230, the Megavitamin colours: ftDr_Init_OnDeath (8014949C)
    /// stores +2234..+2240 only (0x801494C0..CC; ftMr_Init_OnDeath sets both
    /// colours to 9), and ftMr_SpecialN_VitaminRandom (800E0D1C) reads both
    /// before the first pill writes them.
    fn restore_stale_heap(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.specials.vitamin_current = word(0x222C) as i32;
        self.specials.vitamin_previous = word(0x2230) as i32;
    }
    /// ftDr_Init_OnLoad (801494E4): ftMr_Init_OnLoadForDrMario's PUSH_ATTRS.
    /// Retail leaves can_walljump clear. The pill and sheet registrations
    /// (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftDr_Init_OnDeath (8014949C): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset, the Megavitamin colours kept.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.specials.tornado_charged = false;
        self.specials.cape_boosted = false;
        self.taunt = Default::default();
    }
    /// ftCo_Landing_Enter, ftCo_Landing.c:49-52.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.specials.on_landing();
    }
}

/// ftDr_Init_* strings (ftdrmario.c); ftData_Table_Unk0[21] has 303
/// animation rows.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::DrMario,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::DrMario),
    data_file: "PlDr.dat",
    data_symbol: "ftDataDrmario",
    animation_file: "PlDrAJ.dat",
    animation_count: 303,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlDrNr.dat",
            joint_symbol: "PlyDrmario5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDrRe.dat",
            joint_symbol: "PlyDrmario5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDrBu.dat",
            joint_symbol: "PlyDrmario5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDrGr.dat",
            joint_symbol: "PlyDrmario5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDrBk.dat",
            joint_symbol: "PlyDrmario5KBk_Share_joint",
        },
    ],
};

/// take_dmg_cb / death2_cb: whichever of the Tornado's, the sheet's and the
/// taunt pill's callbacks the current motion installed.
fn damage_callback(f: &mut Fighter) {
    ft_mario_family::damage_callback::<DrMario>(f);
    crate::megavitamin::taunt_damage_callback(f);
}

/// ftDr_Init_MotionStateTable[i].x4_flags (read from the retail DOL): the
/// x2070 word each special row sets (ft_800895E0), from action 341.
pub static MOTION_FLAGS: [u32; 10] = [
    0x00000071, 0x00000071, 0x00340111, 0x00340511, 0x00341012, 0x00341012, 0x00340213, 0x00340613,
    0x00340214, 0x00340614,
];
