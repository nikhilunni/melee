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
    /// Aerial-jump turning countdown (mv.co.jumpaerial.x0).
    pub jump_turn_remaining: i32,
    /// Egg Throw scratch (mv.ys.specialhi).
    pub special_hi: crate::special_hi::SpecialHi,
    /// accessory4_cb = fn_8012E644: the Yoshi Bomb landing's stars.
    pub stars_pending: bool,
    /// fp->mv.ys.specials: the Egg Roll scratch.
    pub egg_roll: crate::special_s::EggRoll,
    /// The Egg Roll callbacks and the motion that installed them.
    pub egg_roll_hooks: Option<(melee_ft::fighter::ActionId, crate::special_s::Hooks)>,
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
            stars_pending: false,
            egg_roll: Default::default(),
            egg_roll_hooks: None,
            shield_material_frame: 0.0,
            egg_material_indices: Vec::new(),
            frozen_materials: Vec::new(),
            shield_maximum_health: 0.0,
            special_hi: Default::default(),
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Yoshi>();

impl CharacterCallbacks for Yoshi {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &CHARACTER_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::rows::moves();
    /// Fighter.take_dmg_cb: ftYs_Init_8012BA8C while the Egg Throw's egg is
    /// in hand, fn_8012EDE8 in the Egg Roll; at most one is installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(take_damage);
    /// fn_8012EFF4, installed by the Egg Roll states.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &melee_ft::fighter::assets::FighterAssets)> =
        Some(crate::special_s::hit_dealt);
    /// Fighter.death2_cb: ftYs_Init_8012BA8C (Egg Throw) or fn_8012EC7C (Egg Roll).
    const DEATH: Option<fn(&mut Fighter)> = Some(death);
    /// The Egg Throw's egg (fn_8012E110) and the Yoshi Bomb landing's stars
    /// (fn_8012E644); each runs only in its own motion.
    fn accessory(fighter: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        crate::special_hi::accessory(fighter, assets);
        crate::special_lw::spawn_stars(fighter, assets);
    }
    const CATCH_PULL_START: fn(&mut Fighter, &FighterAssets, f32) -> f32 =
        crate::catch::catch_pull_start;

    fn kind(&self) -> FighterKind {
        FighterKind::Yoshi
    }
    /// ftYs_Init_8012BAC0 (8012BAC0): attribute +0x120.
    fn mouth_capture_scale(&self) -> Option<f32> {
        Some(self.attributes.captured_hurtbox_scale)
    }
    /// ftCo_800DE3FC's FTKIND_YOSHI arm (ftCo_Thrown.c:33) is the shared
    /// mouth-hold path; ftCo_800DD398 has no Yoshi arm.
    fn throw_variant(&self) {}
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
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = crate::special_n::grab;
    /// ftData_SpecialN/S/Hi/Lw[Yoshi]: Egg Lay, Egg Roll, Egg Throw and
    /// Yoshi Bomb.
    fn enter_special(
        fighter: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Up => crate::special_hi::enter(fighter, airborne, assets),
            SpecialSlot::Neutral => crate::special_n::enter(fighter, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(fighter, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(fighter, airborne, assets),
        }
    }
    fn animated_shield(&self) -> bool {
        true
    }
    fn enter_shield(
        fighter: &mut Fighter,
        assets: &FighterAssets,
        reflect: bool,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::enter(fighter, assets, reflect))
    }
    fn animate_shield(fighter: &mut Fighter, assets: &FighterAssets) -> Option<FighterResult<()>> {
        Some(crate::shield::animate(fighter, assets))
    }
    fn input_shield(fighter: &mut Fighter, assets: &FighterAssets) -> Option<FighterResult<()>> {
        // ftYs_GuardOn_1_IASA calls the common GuardReflect IASA verbatim.
        if fighter.core.motion_state.id == melee_types::CommonMotionState::GuardReflect {
            None
        } else {
            Some(crate::shield::input(fighter, assets))
        }
    }
    fn enter_guard_hold(
        fighter: &mut Fighter,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::hold(fighter, assets))
    }
    fn enter_guard_off(fighter: &mut Fighter, assets: &FighterAssets) -> Option<FighterResult<()>> {
        Some(crate::shield::off(fighter, assets))
    }
    fn enter_shield_stun(
        fighter: &mut Fighter,
        impact: &melee_ft::fighter::shield::ShieldImpact,
        assets: &FighterAssets,
    ) -> Option<FighterResult<()>> {
        Some(crate::shield::stun(fighter, impact, assets))
    }
    fn escape_variant(
        fighter: &mut Fighter,
        assets: &FighterAssets,
        rolling: bool,
    ) -> FighterResult<()> {
        crate::shield::escape_entered(fighter, assets, rolling)
    }
    fn escape_finished(fighter: &mut Fighter, assets: &FighterAssets) -> Option<FighterResult<()>> {
        crate::shield::escape_finished(fighter, assets)
    }
    fn escape_animated(fighter: &mut Fighter) {
        if fighter.core.motion_state.id != melee_types::CommonMotionState::EscapeN {
            crate::shield::material(fighter);
        }
    }
    fn aerial_jump_entered(fighter: &mut Fighter) {
        let attr = &fighter.character.get_mut::<Yoshi>().attributes.double_jump;
        fighter.core.combat.armor = attr.armor;
        // retail 800CBFC4: separate fmuls; strict less-than comparison.
        fighter.character.get_mut::<Yoshi>().jump_turn_remaining =
            if fighter.core.input.current.stick.x * fighter.core.physics.facing
                < -attr.reverse_threshold
            {
                attr.turn_frames
            } else {
                0
            };
        Self::aerial_jump_animated(fighter);
    }
    /// ft_800CB6EC (800CB6EC): turn the model, reverse facing halfway through.
    fn aerial_jump_animated(fighter: &mut Fighter) {
        let character = fighter.character.get_mut::<Yoshi>();
        let remaining = &mut character.jump_turn_remaining;
        if *remaining == 0 {
            return;
        }
        *remaining -= 1;
        let frames = character.attributes.double_jump.turn_frames;
        let root = fighter.core.animation.parts[0].joint;
        let old = fighter.core.skeleton.get(root).rotate.y;
        // retail 800CB768 fdivs, 800CB76C fnmsubs; @197 is float PI/180.
        let angle = gekko_math::fma::fnmsubs(0.017453292, 180.0 / frames as f32, old);
        fighter.core.skeleton.set_rotation_y(root, angle);
        if *remaining == frames / 2 {
            fighter.core.physics.facing = -fighter.core.physics.facing;
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
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Yoshi),
    data_file: "PlYs.dat",
    data_symbol: "ftDataYoshi",
    animation_file: "PlYsAJ.dat",
    animation_count: 314,
    part_count: 54,
    part_animation_count: 5,
    additional_part_animations: &[],
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

/// ftYs_Init_MotionStateTable[0..28], contiguous from ftCo_MS_Count (341).
static CHARACTER_ROWS: [melee_ft::fighter::MotionRow; crate::rows::COUNT] = crate::rows::rows();

/// Fighter.take_dmg_cb for whichever special installed one.
fn take_damage(fighter: &mut Fighter) {
    crate::special_hi::drop_egg(fighter);
    crate::special_s::damage_taken(fighter);
}

/// Fighter.death2_cb for whichever special installed one.
fn death(fighter: &mut Fighter) {
    crate::special_hi::drop_egg(fighter);
    crate::special_s::death(fighter);
}
