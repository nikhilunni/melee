//! Ness load/reset hooks, ft/kinds/ftNess/ftness.c.
use crate::attributes::{read_ness_attributes, NessAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    AerialJumpStyle, Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Ness {
    pub attributes: NessAttributes,
    /// ftNs_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +2248, u.ns.bat_gobj: the forward smash's bat is out.
    pub bat: bool,
    /// take_dmg_cb / death2_cb = ftNs_Init_OnDamage, installed with an
    /// article until the next motion change.
    pub damage_callbacks: bool,
    /// The accessory4 callback a special installed.
    pub accessory: Accessory,
    /// PK Flash's motion scratch and Ness's hold on the flash.
    pub pk_flash: crate::special_n::PkFlash,
    /// PK Thunder's motion scratch and Ness's hold on the thunder.
    pub pk_thunder: crate::special_hi::PkThunder,
    /// PSI Magnet's motion scratch.
    pub magnet: crate::special_lw::Magnet,
    /// mv+4 while a row that never writes it is current (PK Fire, PSI
    /// Magnet's turnFrames): the predecessor's word, `None` when the port
    /// does not model that state's.
    pub retained_word: Option<f32>,
}

/// accessory4_cb while a special owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftNs_SpecialS_ItemPKFireSpawn: the bolt on the script's throw flag.
    PkFire,
}
impl Ness {
    pub fn new(attributes: NessAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            bat: false,
            damage_callbacks: false,
            accessory: Accessory::None,
            pk_flash: Default::default(),
            pk_thunder: Default::default(),
            magnet: Default::default(),
            retained_word: None,
        }
    }
}

/// ftNs_Init_OnDamage (801148F8): the yo-yo, PK Flash, PK Thunder and the
/// bat let go.
fn damage_callback(f: &mut Fighter) {
    if !f.character.get::<Ness>().damage_callbacks {
        return;
    }
    // ftNs_SpecialN_ItemPKFlushSetNULL clears both callbacks; the rest of
    // this call still runs.
    crate::special_n::orphan_flash(f);
    crate::special_hi::take_damage(f);
    crate::attack_s4::remove_bat(f);
}

/// The Destroyed side of Ness's articles: an article that ends on its own
/// clears its owner's pointer (itNessbat_ClearOwnerRef ->
/// ftNs_AttackS4_ItemNessBatSetNULL).
fn article_destroyed(f: &mut Fighter, kind: melee_types::ItemKind) {
    match kind {
        melee_types::ItemKind::NessBat => f.character.get_mut::<Ness>().bat = false,
        // it_2725_Logic102_Destroyed -> ftNs_SpecialN_SetNULL.
        melee_types::ItemKind::NessPKFlush => crate::special_n::flash_gone(f),
        // it_802AB90C -> ftNs_SpecialHi_ItemPKThunderRemove.
        melee_types::ItemKind::NessPKThunder => crate::special_hi::thunder_gone(f),
        _ => {}
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Ness>();

impl CharacterCallbacks for Ness {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftNs_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftNs_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    /// None of ftNs_Init_MotionStateTable's 36 rows (0x803CC650) sets x9_b1
    /// (bit 22 of the word at +8): no Ness row starts the KO countdown.
    const KO_COUNTDOWN_ROWS: &'static [u16] = &[];
    /// ftData_SpecialN/S/Hi/Lw[Ness] and the aerial tables.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        match slot {
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// Fighter_8006C80C: the special's accessory4 while installed.
    fn accessory(f: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Ness>().accessory {
            Accessory::PkFire => crate::special_s::fire(f),
            Accessory::None => {}
        }
    }
    /// ftData_OnAbsorb[FTKIND_NESS]: ftNs_Init_OnAbsorb (8011493C).
    const ON_ABSORB: Option<
        fn(&mut Fighter, &FighterAssets, melee_ft::fighter::absorb::Absorbed),
    > =
        Some(crate::special_lw::on_absorb);
    /// mv+4 of the rows that leave it as their predecessor did.
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| {
        let ness = state.get::<Self>();
        match action.0 {
            // mv.ns.specialn.frames_to_loop_charge_air.
            348..=355 => Some(f32::from_bits(ness.pk_flash.gone_frames as u32)),
            // mv.ns.specialhi.thunderTimerLoop1.
            358..=366 => Some(f32::from_bits(ness.pk_thunder.loop_frames as u32)),
            // PK Fire and PSI Magnet never write the word.
            356 | 357 | 367..=376 => ness.retained_word,
            _ => None,
        }
    };
    /// What Ness's articles read of him: the flash, whether he still
    /// holds it in the hold row (ftNs_SpecialN_CheckSpecialNHold).
    fn item_owner(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
        melee_it::ItemOwner {
            illusion: None,
            position: f.physics.position,
            facing: f.physics.facing,
            hold_position: f.physics.position,
            blaster_action: 9,
            remove_blaster: true,
            motion: f.motion_state.action.0,
            articles_fired: 0,
            charge: None,
            holds_needles: false,
            stick: hsd_types::Vec2::new(f.input.current.stick.x, f.input.current.stick.y),
            steering_article: crate::special_n::holding(f),
            detonating_article: false,
            motion_flags: f.motion_flags(),
            in_hitlag: f.core.in_hitlag(),
            anchor: f.physics.position,
            article_stage: None,
            model_scale: f.player.scale * f.attributes.size.model_scaling,
        }
    }
    /// ftCo_AttackS4.c decideFighter: ftNs_AttackS4_Enter.
    const FORWARD_SMASH: Option<melee_ft::fighter::RngEntry> = Some(crate::attack_s4::enter);
    /// ftCo_AttackHi4_CheckInput / ftCo_AttackLw4_CheckInput's FTKIND_NESS
    /// arms: the yo-yo smashes (rows 342..347) and their article
    /// (itnessyoyo.c) are not ported.
    const VERTICAL_SMASH: Option<melee_ft::fighter::VerticalSmash> = Some(|_, up, _| {
        if up {
            unimplemented!("ftnessattackhi4.c:602: ftNs_AttackHi4_Enter (the yo-yo up smash)")
        } else {
            unimplemented!("ftnessattacklw4.c:19: ftNs_AttackLw4_Enter (the yo-yo down smash)")
        }
    });
    /// ftColl_CreateReflectHit(gobj, &xB8_BASEBALL_BAT, ftNs_AttackS4_OnReflect).
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(crate::attack_s4::reflector_contact);
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(crate::attack_s4::reflect_hit);
    /// ftCommon_8007DB58: take_dmg_cb (ftNs_Init_OnDamage) when installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(damage_callback);
    /// ftCo_800D331C: death2_cb, the same callback.
    const DEATH: Option<fn(&mut Fighter)> = Some(damage_callback);
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = article_destroyed;
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.damage_callbacks = false;
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Ness
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_ness_attributes(data)?))
    }
    /// ftCo_JumpAerial_CheckInput's FTKIND_NESS arm: ftNs_JumpAerial_Enter.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Ness
    }
    /// ftNs_Init_OnLoad (8011480C): PUSH_ATTRS. The eleven item
    /// registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftNs_Init_OnDeath (801147C0): ftParts_80074A4C(gobj, 0, 0) and the
    /// article pointers.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.bat = false;
        self.pk_flash.flash_out = false;
        self.pk_thunder.thunder_out = false;
    }
}

/// ftNs_Init_* strings (ftness.c); ftData_Table_Unk0[8] has 326 animation
/// rows. PlCo ftPartsTable maps Ness's 63 joints to 54 parts; ftData.x1C
/// holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Ness,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Ness),
    data_file: "PlNs.dat",
    data_symbol: "ftDataNess",
    animation_file: "PlNsAJ.dat",
    animation_count: 326,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlNsNr.dat",
            joint_symbol: "PlyNess5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNsYe.dat",
            joint_symbol: "PlyNess5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNsBu.dat",
            joint_symbol: "PlyNess5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNsGr.dat",
            joint_symbol: "PlyNess5KGr_Share_joint",
        },
    ],
};
