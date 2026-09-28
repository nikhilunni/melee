//! Samus load/reset hooks, ft/kinds/ftSamus/ftsamus.c.
use crate::attributes::{read_samus_attributes, SamusAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_ef::request::EffectRequest;
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Samus {
    pub attributes: SamusAttributes,
    /// ftSs_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +2230, x2230: the Charge Shot level kept between shots.
    pub charge_level: i32,
    /// Fighter +2234, x2234: the charge's effects are alive.
    pub charge_effects: bool,
    /// Fighter +2238, x2238: missiles fired (ftSs_SpecialS_8012A074).
    pub missiles_fired: u32,
    /// Fighter +2244, x2244: the Screw Attack's effect is alive.
    pub screw_effect: bool,
    /// The morph ball's capsule is installed (mv.co.escape.x4 during a
    /// roll, mv.ss.unk2/unk6.x0 in the bomb rows).
    pub ball: bool,
    /// take_dmg_cb / death2_cb = ftSs_Init_80128428, installed by a
    /// special until the next motion change.
    pub damage_callbacks: bool,
    /// Screw Attack's motion scratch.
    pub screw_attack: crate::special_hi::ScrewAttack,
}
impl Samus {
    pub fn new(attributes: SamusAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            charge_level: 0,
            charge_effects: false,
            missiles_fired: 0,
            screw_effect: false,
            ball: false,
            damage_callbacks: false,
            screw_attack: Default::default(),
        }
    }
}

/// ftSamus_updateDamageDeathCBs (ftSamus/inlines.h): take_dmg_cb and
/// death2_cb = ftSs_Init_80128428 until the next motion change.
pub fn install_damage_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Samus>().damage_callbacks = true;
}

/// ftSs_Init_80128428 (80128428): the charge shot and its effects go and
/// the kept charge resets (ftSs_SpecialN_80129258), every effect goes with
/// the Screw Attack's flag (ftSs_SpecialS_8012A640), and so does the
/// grapple beam (ftCo_800D9C98).
fn damage_callback(f: &mut Fighter) {
    if !f.character.get::<Samus>().damage_callbacks {
        return;
    }
    let samus = f.character.get_mut::<Samus>();
    // ftSamus_UnkAndDestroyAllEF: no charge shot item is modelled yet.
    let charge_effects = std::mem::take(&mut samus.charge_effects);
    samus.charge_level = 0;
    samus.screw_effect = false;
    if charge_effects {
        f.effects.push(EffectRequest::DestroyOwned);
    }
    f.effects.push(EffectRequest::DestroyOwned);
    // ftCo_800D9C98 clears take_dmg_cb and death2_cb.
    f.character.get_mut::<Samus>().damage_callbacks = false;
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Samus>();

impl CharacterCallbacks for Samus {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Samus] and the aerial tables.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        match slot {
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            _ => unimplemented!(
                "ftData_Special{slot:?}[Samus] (airborne: {airborne}): character special entry"
            ),
        }
    }
    /// ftCommon_8007DB58: take_dmg_cb (ftSs_Init_80128428) when installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(damage_callback);
    /// ftCo_800D331C: death2_cb, the same callback.
    const DEATH: Option<fn(&mut Fighter)> = Some(damage_callback);
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.damage_callbacks = false;
    }
    /// ftCo_Catch.c:26-136 (fn_800D952C, fn_800D9558, fn_800D9930): the
    /// grapple beam replaces the body grab.
    fn catch_variant(&mut self) {
        unimplemented!("ftCo_Catch.c:125: Samus grapple beam (fn_800D9558)");
    }

    /// ftCo_800992A8's FTKIND_SAMUS arm: ftCo_80099390.
    const PREPARE_ROLL: Option<fn(&mut Fighter)> = Some(crate::escape::prepare_roll);
    fn escape_variant(
        f: &mut Fighter,
        _assets: &FighterAssets,
        rolling: bool,
    ) -> melee_ft::fighter::assets::Result<()> {
        if rolling {
            crate::escape::roll_entered(f);
        }
        Ok(())
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Samus
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_samus_attributes(data)?))
    }
    /// ftSamus_FighterVars +222C..+2248. The charge-shot and grapple GObjs
    /// (+222C, +223C) are not modelled; a save with either set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.charge_level = word(0x2230) as i32;
        self.charge_effects = word(0x2234) != 0;
        self.missiles_fired = word(0x2238);
        self.screw_effect = word(0x2244) != 0;
        if word(0x222C) != 0 || word(0x223C) != 0 {
            unimplemented!("ftSamus_FighterVars: a saved charge shot or grapple GObj");
        }
    }
    /// ftSs_Init_OnLoad (8012837C): can_walljump and PUSH_ATTRS. The four
    /// item registrations (it_8026B3F8: bomb, charge shot, missile, grapple
    /// beam) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftSs_Init_OnDeath (8012832C): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset (x2234 and x2248 keep their values).
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.charge_level = 0;
        self.missiles_fired = 0;
        self.screw_effect = false;
    }
}

/// ftSs_Init_* strings (ftsamus.c); ftData_Table_Unk0[13] has 313
/// animation rows. PlCo ftPartsTable maps Samus's joints to 54 parts;
/// ftData.x1C holds one part-animation group.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Samus,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Samus),
    data_file: "PlSs.dat",
    data_symbol: "ftDataSamus",
    animation_file: "PlSsAJ.dat",
    animation_count: 313,
    part_count: 54,
    part_animation_count: 1,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlSsNr.dat",
            joint_symbol: "PlySamus5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsPi.dat",
            joint_symbol: "PlySamus5KPi_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsBk.dat",
            joint_symbol: "PlySamus5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsGr.dat",
            joint_symbol: "PlySamus5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsLa.dat",
            joint_symbol: "PlySamus5KLa_Share_joint",
        },
    ],
};
