//! Fox's load/reset hooks, ft/kinds/ftFox/ftfox.c.
use crate::attributes::FoxAttributes;
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
