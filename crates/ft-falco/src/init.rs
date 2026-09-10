//! Falco's load/reset hooks, ft/kinds/ftFalco/ftfalco.c.
use melee_ft::desc::fox_attributes::FoxAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};

/// Character-owned state. The item resource registrations do not spawn items.
#[derive(Clone, Debug)]
pub struct Falco {
    pub attributes: FoxAttributes,
    /// u.fx.x222C_blasterGObj; no blaster exists during Wait.
    pub blaster_present: bool,
    /// ftParts_80074A4C(gobj, 0, 0), OnDeath: default model group state.
    pub model_group: i32,
    pub registered_items: Vec<ItemKind>,
}
impl Falco {
    pub fn new(attributes: FoxAttributes) -> Self {
        Self {
            attributes,
            blaster_present: false,
            model_group: 0,
            registered_items: Vec::new(),
        }
    }
}
impl CharacterCallbacks for Falco {
    fn kind(&self) -> FighterKind {
        FighterKind::Falco
    }
    fn descriptor() -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_falco_attributes(data)?))
    }
    /// ftFc_Init_OnLoad (80149CC4), ftfalco.c:467-484. OnLoadForFalco
    /// (800E576C) copies the shared layout. PUSH_ATTRS is the
    /// owned FoxAttributes copy; item definitions are registered, not spawned.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
        self.registered_items = vec![
            self.attributes.blaster.shot_item_kind,
            self.attributes.blaster.gun_item_kind,
            ItemKind::FalcoPhantasm,
        ];
    }
    /// ftFc_Init_OnDeath (80149ACC), ftfalco.c:440-445; called at cold spawn.
    fn on_reset(&mut self) {
        self.blaster_present = false;
        self.model_group = 0;
    }
}

/// ftFc_Init_* strings, ftData_Table_Unk0[22], and PlCo ftPartsTable[22].
pub const DESCRIPTOR: melee_ft::fighter::assets::CharacterDescriptor =
    melee_ft::fighter::assets::CharacterDescriptor {
        kind: FighterKind::Falco,
        common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Falco),
        data_file: "PlFc.dat",
        data_symbol: "ftDataFalco",
        animation_file: "PlFcAJ.dat",
        animation_count: 327,
        part_count: 54,
        part_animation_count: 5,
        additional_motions: &[],
        costumes: &[
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFcNr.dat",
                joint_symbol: "PlyFalco5K_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFcRe.dat",
                joint_symbol: "PlyFalco5KRe_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFcBu.dat",
                joint_symbol: "PlyFalco5KBu_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlFcGr.dat",
                joint_symbol: "PlyFalco5KGr_Share_joint",
            },
        ],
    };
