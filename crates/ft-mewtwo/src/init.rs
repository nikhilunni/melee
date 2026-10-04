//! Mewtwo load/reset hooks, ft/kinds/ftMewtwo/ftmewtwo.c.
use crate::attributes::{read_mewtwo_attributes, MewtwoAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    AerialJumpStyle, Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Mewtwo {
    pub attributes: MewtwoAttributes,
    /// ftMt_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +2234, u.mt.x2234_shadowBallCharge: Shadow Ball's stored
    /// charge, kept between states until the shot, a hit taken or a death.
    pub shadow_ball_charge: i32,
    /// Fighter +223C, u.mt.x223C_isConfusionBoost: the aerial Confusion
    /// already lifted Mewtwo this airtime.
    pub confusion_boost_used: bool,
    /// The accessory4 callback a special installed.
    pub accessory: Accessory,
    /// Teleport's motion scratch (mv.mt.SpecialHi).
    pub teleport: crate::special_hi::Teleport,
    /// Fighter +222C, u.mt.x222C_disableGObj: Disable's projectile is out
    /// and still Mewtwo's.
    pub disable_article: bool,
    /// take_dmg_cb = ftMt_Init_OnTakeDamage and death2_cb =
    /// ftMt_Init_OnDeath2, installed by a special until the next motion
    /// change.
    pub damage_callbacks: bool,
    /// Fighter +2230, u.mt.x2230_shadowHeldGObj: the Shadow Ball in
    /// Mewtwo's hand.
    pub held_ball: bool,
    /// Shadow Ball's motion scratch (mv.mt.SpecialN).
    pub shadow_ball: crate::special_n::ShadowBall,
    /// Confusion's motion scratch (mv.mt.SpecialS) and reflector.
    pub confusion: crate::special_s::Confusion,
    /// x2222_b2, set by Confusion's grab until a motion change without
    /// Ft_MF_Unk19: a cape hit does not turn Mewtwo round.
    pub cape_turn_blocked: bool,
}

/// accessory4_cb while a special owns it; a motion change removes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftMt_SpecialHi_CreateGFX: Teleport's vanishing flash.
    TeleportStart,
    /// ftMt_SpecialHi_SetEndGFX: Teleport's reappearance.
    TeleportReappear,
    /// ftMt_SpecialLw_CreateDisable: Disable's projectile on the script's
    /// flag; it stays installed through the special.
    Disable,
    /// ftMt_SpecialS_ReflectThink: Confusion's reflector on the script's
    /// flag; it stays installed through the special.
    ConfusionReflect,
}
impl Mewtwo {
    pub fn new(attributes: MewtwoAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            shadow_ball_charge: 0,
            confusion_boost_used: false,
            accessory: Accessory::None,
            teleport: Default::default(),
            disable_article: false,
            damage_callbacks: false,
            held_ball: false,
            shadow_ball: Default::default(),
            confusion: Default::default(),
            cape_turn_blocked: false,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Mewtwo>();

/// Fighter_OnKnockbackEnter/Exit(gobj, 2): texture slots 2 then 0
/// (ftAnim_800704F0).
fn set_knockback_texture_frames(fighter: &mut Fighter, frame: f32) {
    for index in [2, 0] {
        fighter.commands.set_texture_frame(index, frame);
    }
}

/// ftMt_Init_OnTakeDamage (80144F18), the take_dmg_cb: Disable's projectile
/// goes (ftMt_SpecialLw_RemoveDisable), then ftMt_SpecialN_OnDeath.
fn take_damage_callback(f: &mut Fighter) {
    if !f.character.get::<Mewtwo>().damage_callbacks {
        return;
    }
    crate::special_lw::remove_projectile(f);
    // ftMt_SpecialN_OnDeath (80146ED0): the held ball goes; a charge short
    // of full is lost with its effects (ftCo_800BFFAC, efLib_DestroyAll).
    crate::special_n::remove_ball(f);
    let mewtwo = f.character.get::<Mewtwo>();
    if mewtwo.shadow_ball_charge as f32 != mewtwo.attributes.shadow_ball.full_charge {
        crate::special_n::set_charge(f, 0);
        f.effects
            .push(melee_ef::request::EffectRequest::DestroyOwned);
    }
}

/// ftMt_Init_OnDeath2 (80144EE4), the death2_cb: Disable's projectile goes,
/// then ftMt_SpecialN_OnTakeDamage.
fn death_callback(f: &mut Fighter) {
    if !f.character.get::<Mewtwo>().damage_callbacks {
        return;
    }
    crate::special_lw::remove_projectile(f);
    // ftMt_SpecialN_OnTakeDamage (80146E30): the held ball goes and the
    // charge is lost with its effects whatever it was.
    crate::special_n::remove_ball(f);
    crate::special_n::set_charge(f, 0);
    f.effects
        .push(melee_ef::request::EffectRequest::DestroyOwned);
}

impl CharacterCallbacks for Mewtwo {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftMt_Init_OnKnockbackEnter (80145018).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| set_knockback_texture_frames(fighter, 3.0);
    /// ftMt_Init_OnKnockbackExit (8014505C).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| set_knockback_texture_frames(fighter, 0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Mewtwo] and the aerial tables.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// Fighter_8006C80C: the special's accessory4 while installed.
    fn accessory(f: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Mewtwo>().accessory {
            Accessory::TeleportStart => crate::special_hi::vanish_flash(f),
            Accessory::TeleportReappear => crate::special_hi::reappear(f),
            Accessory::Disable => crate::special_lw::create_projectile(f),
            Accessory::ConfusionReflect => crate::special_s::reflect_think(f),
            Accessory::None => {}
        }
    }
    /// itMewtwoDisable_Logic67_Destroyed -> ftMt_SpecialLw_ClearDisableGObj
    /// (80146198).
    /// it_2725_Logic101_Destroyed -> ftMt_SpecialN_SetNULL (80146DC8) for a
    /// ball that never left the hand.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = |f, kind| {
        if kind == melee_types::ItemKind::MewtwoDisable {
            f.character.get_mut::<Mewtwo>().disable_article = false;
        }
        if kind == melee_types::ItemKind::MewtwoShadowBall {
            f.character.get_mut::<Mewtwo>().held_ball = false;
        }
    };
    /// The held Shadow Ball reads Mewtwo's charge while he keeps it
    /// (ftMt_SpecialN_GetChargeLevel).
    fn item_owner(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
        let mewtwo = f.character.get::<Mewtwo>();
        let charge = mewtwo
            .held_ball
            .then(|| (mewtwo.shadow_ball_charge, crate::special_n::full_charge(f)));
        melee_it::ItemOwner {
            illusion: None,
            position: f.physics.position,
            facing: f.physics.facing,
            hold_position: f.physics.position,
            blaster_action: 9,
            remove_blaster: true,
            motion: f.motion_state.action.0,
            articles_fired: 0,
            charge,
            holds_needles: false,
            stick: hsd_types::Vec2::new(f.input.current.stick.x, f.input.current.stick.y),
            steering_article: false,
            detonating_article: false,
            motion_flags: f.motion_flags(),
            in_hitlag: f.core.in_hitlag(),
            anchor: f.physics.position,
            article_stage: None,
            model_scale: f.player.scale * f.attributes.size.model_scaling,
        }
    }
    /// ftCo_ThrowF_Anim (800DD7DC): ftMt_SpecialN_Shoot, the forward
    /// throw's Shadow Balls.
    const THROW_ANIMATION: fn(&mut Fighter, &FighterAssets) = |f, assets| {
        if f.motion_state.id == melee_types::CommonMotionState::ThrowF {
            crate::special_n::throw_shot(f, assets);
        }
    };
    /// ftCommon_8007DB58: take_dmg_cb (ftMt_Init_OnTakeDamage, 80144F18)
    /// when installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(take_damage_callback);
    /// ftCo_800D331C: death2_cb (ftMt_Init_OnDeath2, 80144EE4) when
    /// installed.
    const DEATH: Option<fn(&mut Fighter)> = Some(death_callback);
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.damage_callbacks = false;
        self.cape_turn_blocked = false;
    }
    /// Fighter_UnkProcessGrab: Confusion's grab_cb (ftMt_SpecialS_SetFlags)
    /// and grabbed_cb (ftCo_800BCF18 / ftCo_800BD000).
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = crate::special_s::grab;
    /// ftMewtwo_SetGrabVictim: ftCo_800DE2A8, then ftCo_80090780.
    const SPECIAL_RELEASE: melee_ft::fighter::SpecialRelease =
        melee_ft::fighter::capture_mewtwo::release;
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(crate::special_s::reflector_contact);
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(crate::special_s::reflect_hit);
    /// ftMt_SpecialS_ReflectThink sets x2218_b4 after ftColl_CreateReflectHit.
    const REFLECTOR_KEEPS_OWNER: bool = true;
    /// ftCo_800C3538's x2222_b2 (ftMt_SpecialS_SetFlags).
    const CAPE_TURN_BLOCKED: fn(&mut Fighter) -> bool =
        |f| f.character.get::<Mewtwo>().cape_turn_blocked;

    fn kind(&self) -> FighterKind {
        FighterKind::Mewtwo
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_mewtwo_attributes(data)?))
    }
    /// ftMewtwo_FighterVars +222C..+223F. The three article GObjs (+222C
    /// Disable, +2230 the held Shadow Ball, +2238 the shot one) are not
    /// restored; a save with one set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.shadow_ball_charge = word(0x2234) as i32;
        self.confusion_boost_used = word(0x223C) != 0;
        if word(0x222C) != 0 || word(0x2230) != 0 || word(0x2238) != 0 {
            unimplemented!("ftMewtwo_FighterVars: a saved Disable or Shadow Ball GObj");
        }
        if self.shadow_ball_charge as f32 == self.attributes.shadow_ball.full_charge {
            // The glow is the fighter's secondary colour slot, which the
            // character payload cannot restore.
            unimplemented!("ftMt_Init_UnkMotionStates4: a savestate with a full Shadow Ball");
        }
    }
    /// ftMt_Init_OnLoad (80144E48): PUSH_ATTRS, parts[FtPart_TransN].flags_b4
    /// and x2221_b2 (the model follows TransN's extracted motion). The two
    /// item registrations (it_8026B3F8: Disable, Shadow Ball) belong to the
    /// item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.compensates_root_motion = true;
    }
    /// ftMt_Init_OnDeath (80144DFC): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.shadow_ball_charge = 0;
        self.confusion_boost_used = false;
    }
    /// ftCo_Landing_Enter's FTKIND_MEWTWO arm (ftCo_Landing.c:77-79).
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.confusion_boost_used = false;
    }
    /// ftMt_JumpAerial_Enter (800CC238): the animation carries the rise.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Mewtwo
    }
    /// ftCo_8009DD94's FTKIND_MEWTWO arm (ftdynamics.c:404-408): every
    /// chain's forces start at the ninth bone.
    fn dynamics_first_force_bone(&self, _set: usize, _count: usize) -> usize {
        8
    }
}

/// ftMt_Init_* strings (ftmewtwo.c); ftData_Table_Unk0[16] has 314 animation
/// rows. PlCo ftPartsTable maps Mewtwo's joints to 54 parts; ftData.x1C
/// holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Mewtwo,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mewtwo),
    data_file: "PlMt.dat",
    data_symbol: "ftDataMewtwo",
    animation_file: "PlMtAJ.dat",
    animation_count: 314,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlMtNr.dat",
            joint_symbol: "PlyMewtwo5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtRe.dat",
            joint_symbol: "PlyMewtwo5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtBu.dat",
            joint_symbol: "PlyMewtwo5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtGr.dat",
            joint_symbol: "PlyMewtwo5KGr_Share_joint",
        },
    ],
};
