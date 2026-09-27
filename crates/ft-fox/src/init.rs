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
    fn enter_special(
        fighter: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) {
        ft_fox_family::enter_special::<Self>(fighter, slot, airborne, assets);
    }
    fn retained_scratch_word(&self, action: melee_ft::fighter::ActionId) -> Option<f32> {
        ft_fox_family::special_lw::retained_scratch_word(&self.special_lw, action)
            .or_else(|| ft_fox_family::special_s::retained_scratch_word(&self.special_side, action))
    }
    fn accessory(
        fighter: &mut melee_ft::fighter::Fighter,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) {
        ft_fox_family::special_n::accessory::<Self>(fighter, assets);
        ft_fox_family::special_s::accessory::<Self>(fighter, assets);
        ft_fox_family::special_hi::accessory::<Self>(fighter, assets);
        ft_fox_family::special_lw::accessory::<Self>(fighter, assets);
    }
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(ft_fox_family::special_lw::reflector_contact::<Self>);
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
        additional_motions: &[
            214, // ftCo_SM_StopCeil.
            47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 65, 66, 67, 165, 166, 170, 171, 172,
            173, 174, 175, 176, 177, 179, 180, 183, 184, 29, 62, 178, 191, 192, 201, 244, 248, 254,
            255, 263, 219, 221, 223, 239, 240, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304,
            305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320,
        ],
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
