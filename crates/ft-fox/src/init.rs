//! Fox's load/reset hooks, ft/kinds/ftFox/ftfox.c.
use melee_ft::desc::fox_attributes::FoxAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};

/// Character-owned state. The item resource registrations do not spawn items.
#[derive(Clone, Debug)]
pub struct Fox {
    pub attributes: FoxAttributes,
    /// u.fx.x222C_blasterGObj; no blaster exists during Wait.
    pub blaster_present: bool,
    /// ftParts_80074A4C(gobj, 0, 0), OnDeath: default model group state.
    pub model_group: i32,
    pub registered_items: Vec<ItemKind>,
}
impl Fox {
    pub fn new(attributes: FoxAttributes) -> Self {
        Self {
            attributes,
            blaster_present: false,
            model_group: 0,
            registered_items: Vec::new(),
        }
    }
}
impl CharacterCallbacks for Fox {
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
        self.blaster_present = false;
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
        additional_motions: &[
            47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 65, 66, 67, 165, 166, 170, 171, 172,
            173, 174, 175, 176, 177, 179, 180, 183, 184, 29, 62, 178, 191, 192, 201, 244, 248, 254,
            255, 263,
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
