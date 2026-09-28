//! Link load/reset hooks, ft/kinds/ftLink/ftlink.c.
use crate::attributes::{read_link_attributes, LinkAttributes};
use ft_link_family::{FamilyState, LinkFamily, Specials};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow, PlayerSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Link {
    pub attributes: LinkAttributes,
    /// ftLk_Init_OnDeath resets model groups 0, 1 and 2 to selection 0.
    pub model_groups: [i32; 3],
    /// ftLk_FighterVars, mv.lk and the callbacks the states install.
    pub specials: Specials,
}
impl Link {
    pub fn new(attributes: LinkAttributes) -> Self {
        Self {
            attributes,
            model_groups: [0; 3],
            specials: Specials::default(),
        }
    }
}
impl LinkFamily for Link {
    fn attributes(&self) -> &LinkAttributes {
        &self.attributes
    }
    const SPIN_TIP_PART: melee_types::FtPart = melee_types::FtPart::L2ndNa;
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [MotionRow; FamilyState::COUNT] = ft_link_family::rows::<Link>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Link>();

impl CharacterCallbacks for Link {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftLk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftLk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    /// ftParts_800753D4(fp, Fighter_804D6540[kind]->x0, item_list[6]): the
    /// shield bone (part 68 for Link, 72 for Young Link).
    const ONLOAD_ITEM_JOINT: Option<u32> = Some(6);
    /// ftCo_800CED30: ftLk_MS_AttackS42.
    const FORWARD_SMASH_COMBO: Option<melee_ft::fighter::ActionId> =
        Some(FamilyState::AttackS42.action());
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_link_family::special_moves();
    const MOTION_FLAGS: &'static [u32] = &ft_link_family::motion_flags(false);

    /// ftData_SpecialN/S/Hi/Lw and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        ft_link_family::enter_special::<Self>(f, slot, airborne, assets);
    }
    /// Fighter_8006C80C: the state's one-shot accessory4.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        ft_link_family::accessory::<Self>(f, assets);
    }
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = ft_link_family::retained_scratch_word::<Self>;
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(ft_link_family::take_damage::<Self>);
    /// ftLk_AttackAir_Enter (ftCo_AttackAir.c decideFighter).
    const ENTER_AERIAL: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        ft_link_family::attack_air::enter::<Self>;
    /// The down aerial's lwOnHit.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> =
        Some(ft_link_family::attack_air::deal_damage::<Self>);
    const DEATH: Option<fn(&mut Fighter)> = Some(ft_link_family::death::<Self>);
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) =
        ft_link_family::article_destroyed::<Self>;
    const ARTICLE_REQUEST: fn(
        &mut Fighter,
        &FighterAssets,
        melee_types::ItemKind,
        melee_it::OwnerRequest,
    ) -> Option<u8> = ft_link_family::article_request::<Self>;
    fn item_owner(f: &mut Fighter, assets: &FighterAssets) -> melee_it::ItemOwner {
        ft_link_family::item_owner::<Self>(f, assets)
    }
    /// Fighter_ChangeMotionState, fighter.c:1376-1389.
    fn on_motion_change(&mut self) {
        ft_link_family::motion_changed(&mut self.specials);
    }
    /// ftCo_800C3B10 / ftCo_800C3BE8: the aerial hookshot.
    const AIR_TETHER: Option<fn(&mut Fighter, &FighterAssets) -> bool> =
        Some(ft_link_family::air_tether::<Self>);
    /// ftCo_800D8C54: mv+0, the hookshot's frame count, restarts.
    fn catch_variant(&mut self) {
        ft_link_family::hookshot::restart_count::<Self>(&mut self.specials);
    }
    /// ftCo_0D8E.c and the catch states' Link arms: the grabs throw the
    /// hookshot and pull through it.
    const TETHER: Option<melee_ft::fighter::tether::Tether> =
        Some(ft_link_family::tether::<Self>());
    fn kind(&self) -> FighterKind {
        FighterKind::Link
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_link_attributes(data)?))
    }
    /// ftLk_Init_OnLoad (800EAE44): PUSH_ATTRS and five item registrations
    /// (bomb, boomerang, hookshot, arrow, bow; it_8026B3F8); ftdata.c
    /// supplies all four specials. ftParts_800753D4 attaches item_list[6]
    /// to the model.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// The down aerial's hit end frame OnLoad reads from animation 72.
    fn on_resources_loaded(&mut self, assets: &FighterAssets, _player: &PlayerSlot) {
        self.attributes = self
            .attributes
            .clone()
            .with_down_air_end(ft_link_family::down_air_frames(assets));
    }
    /// ftLk_Init_OnDeath (800EAD84): ftParts_80074A4C(gobj, 0..2, 0) and
    /// the FighterVars reset.
    fn on_reset(&mut self) {
        self.model_groups = [0; 3];
        self.specials.reset();
    }
}

/// ftLk_Init_* strings; ftData_Table_Unk0[6] has 314 animation rows
/// (ftCo_SM_Count plus ftLk_SM_SelfCount).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Link,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Link),
    data_file: "PlLk.dat",
    data_symbol: "ftDataLink",
    animation_file: "PlLkAJ.dat",
    animation_count: 314,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlLkNr.dat",
            joint_symbol: "PlyLink5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLkRe.dat",
            joint_symbol: "PlyLink5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLkBu.dat",
            joint_symbol: "PlyLink5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLkBk.dat",
            joint_symbol: "PlyLink5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLkWh.dat",
            joint_symbol: "PlyLink5KWh_Share_joint",
        },
    ],
};
