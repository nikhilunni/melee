//! Fox's load/reset hooks, ft/kinds/ftFox/ftfox.c.
use melee_ft::desc::fox_attributes::FoxAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};

/// Character-owned state. The item resource registrations do not spawn items.
#[derive(Clone, Debug)]
pub struct Fox {
    pub attributes: FoxAttributes,
    pub special_neutral: ft_fox_family::SpecialNeutral,
    pub special_lw: ft_fox_family::special_lw::SpecialLw,
    pub special_hi: ft_fox_family::special_hi::SpecialHi,
    pub special_side: ft_fox_family::special_s::SpecialSide,
    /// ftParts_80074A4C(gobj, 0, 0), OnDeath: default model group state.
    pub model_group: i32,
    pub registered_items: Vec<ItemKind>,
}
impl Fox {
    pub fn new(attributes: FoxAttributes) -> Self {
        Self {
            attributes,
            special_neutral: Default::default(),
            special_side: Default::default(),
            special_hi: Default::default(),
            special_lw: Default::default(),
            model_group: 0,
            registered_items: Vec::new(),
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Fox>();

impl CharacterCallbacks for Fox {
    /// No special reads fp->item_gobj; a held item stays in hand.
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const KNOCKBACK_ENTER: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = ft_fox_family::knockback_enter;
    const KNOCKBACK_EXIT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = ft_fox_family::knockback_exit;
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    fn throw_variant(&self) {}
    const THROW_ANIMATION: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = ft_fox_family::special_n::throw_animation::<Self>;
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_fox_family::special_moves();
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &ft_fox_family::rows::<Self>();
    const MOTION_FLAGS: &'static [u32] = &MOTION_FLAGS;
    fn enter_special(
        fighter: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) {
        ft_fox_family::enter_special::<Self>(fighter, slot, airborne, assets);
    }
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| {
        let scratch = state.get::<Self>();
        ft_fox_family::special_lw::retained_scratch_word(&scratch.special_lw, action).or_else(
            || ft_fox_family::special_s::retained_scratch_word(&scratch.special_side, action),
        )
    };
    fn accessory(
        fighter: &mut melee_ft::fighter::Fighter,
        assets: &melee_ft::fighter::assets::FighterAssets,
        _rng: &mut gekko_math::HsdRng,
    ) {
        ft_fox_family::special_n::accessory::<Self>(fighter, assets);
        ft_fox_family::special_s::accessory::<Self>(fighter, assets);
        ft_fox_family::special_hi::accessory::<Self>(fighter, assets);
        ft_fox_family::special_lw::accessory::<Self>(fighter, assets);
    }
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(ft_fox_family::special_lw::reflector_contact::<Self>);
    /// Illusion's x2222_b2 (ftCo_800C3538).
    const CAPE_TURN_BLOCKED: fn(&mut melee_ft::fighter::Fighter) -> bool =
        ft_fox_family::special_s::cape_turn_blocked::<Self>;
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(ft_fox_family::special_lw::reflect_hit::<Self>);
    const TAKE_DAMAGE: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(ft_fox_family::special_n::remove_blaster::<Self>);
    /// ftFx_Init_800E5588 is also the Blaster's death2_cb.
    const DEATH: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(ft_fox_family::special_n::remove_blaster::<Self>);
    fn item_owner(
        fighter: &mut melee_ft::fighter::Fighter,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_it::ItemOwner {
        ft_fox_family::special_s::item_owner::<Self>(fighter, assets)
    }
    fn item_muzzle(
        fighter: &mut melee_ft::fighter::Fighter,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) -> Option<(hsd_types::Vec3, f32)> {
        Some(ft_fox_family::special_n::item_muzzle(
            &mut fighter.core,
            assets,
        ))
    }
    fn kind(&self) -> FighterKind {
        FighterKind::Fox
    }
    fn descriptor() -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_fox_attributes(data)?))
    }
    /// ftFx_Init_OnLoad (0x800E57AC), ftfox.c:486-501. PUSH_ATTRS is the
    /// owned FoxAttributes copy; item definitions are registered, not spawned.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
        self.registered_items = vec![
            self.attributes.blaster.shot_item_kind,
            self.attributes.blaster.gun_item_kind,
            ItemKind::FoxIllusion,
        ];
    }
    /// ftFx_Init_OnDeath (0x800E5554), ftfox.c:448-455; called at cold spawn.
    /// Fighter_ChangeMotionState: take_dmg_cb and death2_cb go.
    fn on_motion_change(&mut self) {
        self.special_neutral.callbacks_armed = false;
    }
    fn on_reset(&mut self) {
        self.special_neutral = Default::default();
        self.special_side = Default::default();
        self.special_hi = Default::default();
        self.special_lw = Default::default();
        self.model_group = 0;
    }
}

/// ftFx_Init_* strings, ftData_Table_Unk0[1], and PlCo ftPartsTable[1].
pub const DESCRIPTOR: melee_ft::fighter::assets::CharacterDescriptor =
    melee_ft::fighter::assets::CharacterDescriptor {
        kind: FighterKind::Fox,
        common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Fox),
        data_file: "PlFx.dat",
        data_symbol: "ftDataFox",
        animation_file: "PlFxAJ.dat",
        animation_count: 327,
        part_count: 54,
        part_animation_count: 5,
        // A3 combat scripts: smash, tumble, prone recovery, tech and linked throws.
        additional_part_animations: ft_fox_family::KNOCKBACK_PART_ANIMATIONS,
        costumes: &[
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFxNr.dat",
                joint_symbol: "PlyFox5K_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFxOr.dat",
                joint_symbol: "PlyFox5KOr_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFxLa.dat",
                joint_symbol: "PlyFox5KLa_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFxGr.dat",
                joint_symbol: "PlyFox5KGr_Share_joint",
            },
        ],
    };

impl ft_fox_family::FoxFamily for Fox {
    const GHOST_ARTICLE_INDEX: u32 = 2;
    const LASER: ItemKind = ItemKind::FoxLaser;
    const BLASTER: ItemKind = ItemKind::FoxBlaster;
    const GHOST: ItemKind = ItemKind::FoxIllusion;
    const SOUNDS: ft_fox_family::FamilySounds = ft_fox_family::FamilySounds {
        fire: [110103, 110106],
        holster: 110100,
        throw_fire: 110109,
    };
    fn attributes(&self) -> &FoxAttributes {
        &self.attributes
    }
    fn special_lw(&mut self) -> &mut ft_fox_family::special_lw::SpecialLw {
        &mut self.special_lw
    }
    fn special_hi(&mut self) -> &mut ft_fox_family::special_hi::SpecialHi {
        &mut self.special_hi
    }
    fn special_side(&mut self) -> &mut ft_fox_family::special_s::SpecialSide {
        &mut self.special_side
    }
    fn special_neutral(&mut self) -> &mut ft_fox_family::SpecialNeutral {
        &mut self.special_neutral
    }
}

/// ftFx_Init_MotionStateTable[i].x4_flags (read from the retail DOL): the x2070 word each
/// special row sets (ft_800895E0), from action 341.
pub static MOTION_FLAGS: [u32; 35] = [
    0x00340111, 0x003C0111, 0x00340111, 0x00340511, 0x003C0511, 0x00340511, 0x00340212, 0x00340212,
    0x00340212, 0x00340612, 0x00340612, 0x00340612, 0x00340213, 0x00340613, 0x00340213, 0x00340613,
    0x00340213, 0x00340613, 0x00340613, 0x00341014, 0x003C1014, 0x00341014, 0x00341014, 0x00341014,
    0x00341414, 0x003C1414, 0x00341414, 0x00341414, 0x00341414, 0x00000072, 0x00000072, 0x00000072,
    0x00000072, 0x00000072, 0x00000072,
];
