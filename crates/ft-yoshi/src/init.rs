//! ftYs_Init_OnLoad / OnDeath, ft/kinds/ftYoshi/ftyoshi.c.
use crate::attributes::YoshiAttributes;
use hsd_types::Vec3;
use melee_ft::fighter::assets::{
    CharacterDescriptor, CostumeDescriptor, FighterAssets, Result as FighterResult,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks, Fighter, PlayerSlot};
use melee_types::{FighterKind, ItemKind};
#[derive(Clone, Debug)]
pub struct Yoshi {
    pub attributes: YoshiAttributes,
    /// Fighter +222C: pre-Egg Roll model scale, preserved by OnDeath.
    pub egg_roll_scale: Vec3,
    /// Fighter +2238: Egg Throw item handle; reset by OnDeath.
    pub egg_active: bool,
    pub registered_items: Vec<ItemKind>,
    pub model_group: i32,
    /// Costume visibility groups selected by ftParts_8007487C.
    pub egg_material_indices: Vec<Vec<usize>>,
    /// MObj AObjs frozen by ftYs_Init_8012B6E8; renderer consumes this output.
    pub frozen_materials: Vec<usize>,
    pub shield_material_frame: f32,
    pub shield_maximum_health: f32,
    pub egg_body: bool,
    pub egg_hurtbox: Option<melee_ft::fighter::caches::Hurtbox>,
    /// Aerial-jump turning countdown (mv.co.jumpaerial.x0).
    pub jump_turn_remaining: i32,
}
impl Yoshi {
    pub fn new(attributes: YoshiAttributes) -> Self {
        Self {
            attributes,
            egg_roll_scale: Vec3::ZERO,
            egg_active: false,
            registered_items: Vec::new(),
            model_group: 0,
            jump_turn_remaining: 0,
            shield_material_frame: 0.0,
            egg_material_indices: Vec::new(),
            frozen_materials: Vec::new(),
            shield_maximum_health: 0.0,
            egg_body: false,
            egg_hurtbox: None,
        }
    }
}
impl CharacterCallbacks for Yoshi {
    fn special_rows() -> &'static [melee_ft::fighter::MotionRow<Self>] {
        &CHARACTER_ROWS
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Yoshi
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        let mut character = Self::new(crate::attributes::read_yoshi_attributes(data)?);
        character.egg_material_indices = crate::material::egg_material_indices(data)?;
        Ok(character)
    }
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |o| u32::from_be_bytes(raw[o..o + 4].try_into().unwrap());
        self.egg_roll_scale = Vec3::new(
            f32::from_bits(word(0x222C)),
            f32::from_bits(word(0x2230)),
            f32::from_bits(word(0x2234)),
        );
        self.egg_active = word(0x2238) != 0;
    }
    /// ftYs_Init_OnLoad (8012B99C): three item definitions, no walljump.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.grounded_down_bound = true;
        self.registered_items = vec![
            ItemKind::YoshiEggThrow,
            ItemKind::YoshiStar,
            ItemKind::YoshiEggLay,
        ];
    }
    /// ftYs_Init_OnDeath (8012B960): model group 0 and egg handle only.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.egg_active = false;
    }
    fn on_costume_loaded(
        &mut self,
        archive: &hsd_archive::Archive,
        costume: u8,
    ) -> FighterResult<()> {
        self.frozen_materials = self.egg_material_indices[usize::from(costume)].clone();
        let symbol = DESCRIPTOR.costumes[usize::from(costume)]
            .joint_symbol
            .replace("_joint", "_matanim_joint");
        self.attributes.shield_material_frames =
            crate::material::egg_material_frames(archive, &symbol, &self.frozen_materials)?;
        Ok(())
    }
    fn on_resources_loaded(&mut self, assets: &FighterAssets, _player: &PlayerSlot) {
        self.shield_maximum_health = assets.shield_health;
    }
    fn action_id(&self, state: melee_types::CommonMotionState) -> i32 {
        use crate::shield::EggShieldMotion as Egg;
        use melee_types::CommonMotionState as S;
        match state {
            S::GuardOn => Egg::On as i32,
            S::Guard => Egg::Hold as i32,
            S::GuardOff => Egg::Off as i32,
            S::GuardSetOff => Egg::Damage as i32,
            S::GuardReflect => Egg::Reflect as i32,
            _ => state.into(),
        }
    }
    fn check_hurtbox_interaction(&self) {
        if self.egg_body {
            unimplemented!("ftyoshiguard.c:31-48: combat against the egg hurt capsule");
        }
    }
    fn animated_shield(&self) -> bool {
        true
    }
    fn enter_shield(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
        reflect: bool,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::enter(fighter, assets, reflect))
    }
    fn animate_shield(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::animate(fighter, assets))
    }
    fn input_shield(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        // ftYs_GuardOn_1_IASA calls the common GuardReflect IASA verbatim.
        if fighter.motion_state.id == melee_types::CommonMotionState::GuardReflect {
            None
        } else {
            Some(crate::shield::input(fighter, assets))
        }
    }
    fn enter_guard_hold(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::hold(fighter, assets))
    }
    fn enter_guard_off(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::off(fighter, assets))
    }
    fn escape_variant(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
        rolling: bool,
    ) -> FighterResult<()> {
        crate::shield::escape_entered(fighter, assets, rolling)
    }
    fn escape_finished(
        fighter: &mut Fighter<Self>,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        crate::shield::escape_finished(fighter, assets)
    }
    fn escape_animated(fighter: &mut Fighter<Self>) {
        if fighter.motion_state.id != melee_types::CommonMotionState::EscapeN {
            crate::shield::material(fighter);
        }
    }
    fn aerial_jump_entered(fighter: &mut Fighter<Self>) {
        let attr = &fighter.character.attributes.double_jump;
        fighter.combat.armor = attr.armor;
        // retail 800CBFC4: separate fmuls; strict less-than comparison.
        fighter.character.jump_turn_remaining =
            if fighter.input.current.stick.x * fighter.physics.facing < -attr.reverse_threshold {
                attr.turn_frames
            } else {
                0
            };
        Self::aerial_jump_animated(fighter);
    }
    /// ft_800CB6EC (800CB6EC): turn the model, reverse facing halfway through.
    fn aerial_jump_animated(fighter: &mut Fighter<Self>) {
        let remaining = &mut fighter.character.jump_turn_remaining;
        if *remaining == 0 {
            return;
        }
        *remaining -= 1;
        let frames = fighter.character.attributes.double_jump.turn_frames;
        let root = fighter.animation.parts[0].joint;
        let old = fighter.skeleton.get(root).rotate.y;
        // retail 800CB768 fdivs, 800CB76C fnmsubs; @197 is float PI/180.
        let angle = gekko_math::fma::fnmsubs(0.017453292, 180.0 / frames as f32, old);
        fighter.skeleton.set_rotation_y(root, angle);
        if *remaining == frames / 2 {
            fighter.physics.facing = -fighter.physics.facing;
        }
    }
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Yoshi
    }
}
/// ftYs_Init_* strings; ftData_Table_Unk0[14] has 314 motions. PlCo's
/// ftPartsTable[14] maps 70 joints to 54 semantic parts; five part groups.
/// ftData.x2C is an empty dynamic-bone set (zero chains and colliders).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Yoshi,
    data_file: "PlYs.dat",
    data_symbol: "ftDataYoshi",
    animation_file: "PlYsAJ.dat",
    animation_count: 314,
    part_count: 54,
    part_animation_count: 5,
    additional_motions: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlYsNr.dat",
            joint_symbol: "PlyYoshi5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlYsRe.dat",
            joint_symbol: "PlyYoshi5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlYsBu.dat",
            joint_symbol: "PlyYoshi5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlYsYe.dat",
            joint_symbol: "PlyYoshi5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlYsPi.dat",
            joint_symbol: "PlyYoshi5KPi_Share_joint",
        },
        CostumeDescriptor {
            file: "PlYsAq.dat",
            joint_symbol: "PlyYoshi5KAq_Share_joint",
        },
    ],
};

/// ftYs_Init_MotionStateTable[0..5]: egg shield states, actions 341..345.
const CHARACTER_ROWS: [melee_ft::fighter::MotionRow<Yoshi>; 5] = {
    use melee_ft::fighter::{ActionId, CharacterCallbacks, MotionRow};
    use melee_types::CommonMotionState as S;
    [
        MotionRow {
            action: ActionId(341),
            ..Yoshi::COMMON[S::GuardOn as usize]
        },
        MotionRow {
            action: ActionId(342),
            ..Yoshi::COMMON[S::Guard as usize]
        },
        MotionRow {
            action: ActionId(343),
            ..Yoshi::COMMON[S::GuardOff as usize]
        },
        MotionRow {
            action: ActionId(344),
            ..Yoshi::COMMON[S::GuardSetOff as usize]
        },
        MotionRow {
            action: ActionId(345),
            ..Yoshi::COMMON[S::GuardReflect as usize]
        },
    ]
};
