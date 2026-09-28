//! Ganondorf load/reset hooks, ft/kinds/ftGanon/ftganon.c. His motion
//! table is Captain Falcon's callbacks (ftGn_Init_MotionStateTable), with
//! his own effects in their kind switches.
use crate::attributes::{read_ganon_attributes, CaptainAttributes};
use ft_captain_family::{
    special_hi_catch, special_lw, special_s, CaptainFamily, FamilyEffects, Specials,
};
use melee_ft::fighter::assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets};
use melee_ft::fighter::{
    ActionId, Capabilities, CharacterCallbacks, CharacterState, Fighter, MotionRow, SpecialSlot,
};
use melee_types::{FighterKind, FtPart};

#[derive(Clone, Debug)]
pub struct Ganondorf {
    pub attributes: CaptainAttributes,
    /// ftGn_Init_OnDeath: ftParts_80074A4C(gobj, 0, 0) and (gobj, 1, -1).
    pub model_groups: [i32; 2],
    /// The FighterVars flags and mv.ca scratch of the shared specials.
    pub specials: Specials,
}
impl Ganondorf {
    pub fn new(attributes: CaptainAttributes) -> Self {
        Self {
            attributes,
            model_groups: DEATH_MODEL_GROUPS,
            specials: Specials::default(),
        }
    }
}
/// ftGn_Init_OnDeath's model-group selections (8014EC00..30).
const DEATH_MODEL_GROUPS: [i32; 2] = [0, -1];

impl CaptainFamily for Ganondorf {
    /// The FTKIND_GANON arms.
    const EFFECTS: FamilyEffects = FamilyEffects {
        // efSync_Spawn(1291, gobj, parts[FtPart_TopN], parts[78]): efSync
        // 0x50B's two models (0x4A38, 0x4A39).
        punch: 1291,
        punch_bones: [0, 78],
        punch_wind: true,
        // efSync_Spawn(1293, gobj, L2ndNb): efSync 0x50D, model 0x4A3B.
        boost_start: 1293,
        boost_start_part: FtPart::L2ndNb,
        // efSync_Spawn(1294 / 1295, gobj, TransN, &facing_dir): efSync
        // 0x50E / 0x50F (0x4A3C / 0x4A3D), scaled and turned to the facing.
        ground_lunge: 1294,
        air_lunge: 1295,
        // efAsync_Spawn(gobj, &fp->x60C, 3, 0x50C, foot, &angle): efSync
        // 0x50C, model 0x4A3A rotated by the angle each update.
        kick_flame: 0x50C,
    };
    fn attributes(&self) -> &CaptainAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [MotionRow; ft_captain_family::ROW_COUNT] =
    ft_captain_family::special_rows::<Ganondorf>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Ganondorf>();

/// ftGn_Init_OnKnockbackEnter/Exit select these part-animation variants;
/// no subaction script needs to name them.
const KNOCKBACK_PART_ANIMATIONS: &[(usize, usize)] = &[(3, 2), (4, 2), (3, 3), (4, 3)];

/// ftCo_800DEA28's FTKIND_GANON arm: lb_800119DC(&TopN, 80, 1.0, 0.003,
/// 1.0471976) (@201..@203), then ftCo_800DEBD0 twice: the case falls
/// through into the default arm (800DEAB4, 800DEABC).
fn enter_taunt(f: &mut Fighter, assets: &FighterAssets) -> melee_ft::fighter::assets::Result<()> {
    let c = &mut f.core;
    // lb_8000B1CC(fp->parts[0].joint, NULL, &pos).
    let center = melee_ft::fighter::caches::bone_position(
        &mut c.skeleton,
        c.animation.root,
        0,
        hsd_types::Vec3::ZERO,
    );
    c.commands
        .radial_impulses
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: 80,
            strength: 1.0,
            decay: 0.003,
            // @203 = 1.0471976f, FRAC_PI_3 rounded to f32.
            phase_step: std::f32::consts::FRAC_PI_3,
        });
    f.enter_common_taunt(assets)?;
    f.enter_common_taunt(assets)
}

impl CharacterCallbacks for Ganondorf {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        enter_taunt;
    /// ftGn_Init_OnKnockbackEnter (8014EE60): part animations 3 and 4 to
    /// variant 3; no Fighter_OnKnockbackEnter.
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) = |fighter, assets| {
        fighter.apply_part_animation(assets, 3, 3, 0.0);
        fighter.apply_part_animation(assets, 4, 3, 0.0);
    };
    /// ftGn_Init_OnKnockbackExit: back to variant 2.
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) = |fighter, assets| {
        fighter.apply_part_animation(assets, 3, 2, 0.0);
        fighter.apply_part_animation(assets, 4, 2, 0.0);
    };
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_captain_family::special_moves();
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    /// ftData_SpecialN/S/Hi/Lw[Ganon]: the ftCa_*_Enter entries.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, air: bool, a: &FighterAssets) {
        ft_captain_family::enter_special::<Self>(f, slot, air, a);
    }
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> =
        Some(special_lw::deal_damage::<Self>);
    const HURTBOX_DETECT: Option<
        fn(&mut Fighter, &FighterAssets, melee_ft::fighter::damage::InertTouch),
    > = Some(special_s::detect::<Self>);
    /// Dark Dive's catch: ftCa_SpecialLw_800E5128 and ftCo_8009CA0C.
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = special_hi_catch::grab;
    /// accessory4: ftCa_SpecialLw_800E550C while hanging from the victim.
    fn accessory(fighter: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        special_hi_catch::follow_victim(fighter);
    }
    /// ftCa_Init_800E28C8: Gerudo Dragon's take_dmg_cb.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(special_s::remove_effects::<Self>);
    /// ftCa_Init_800E28C8: Gerudo Dragon's death2_cb.
    const DEATH: Option<fn(&mut Fighter)> = Some(special_s::remove_effects::<Self>);
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        ft_captain_family::retained_scratch_word::<Self>;

    fn kind(&self) -> FighterKind {
        FighterKind::Ganon
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_ganon_attributes(data)?))
    }
    /// ftCaptain_FighterVars (types.h): the saved startup and lunge flags.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.specials.restore_saved(raw);
    }
    /// ftGn_Init_OnLoad -> ftCa_Init_OnLoadForGanon: PUSH_ATTRS only, so
    /// no walljump; ftdata.c supplies all four specials.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftGn_Init_OnDeath (8014EBFC): both model groups, then +2230, +222C.
    fn on_reset(&mut self) {
        self.model_groups = DEATH_MODEL_GROUPS;
        self.specials.reset();
    }
}

/// ftGn_Init_* strings; ftData_Table_Unk0[25] has 318 animation rows
/// (ftCo_SM_Count plus ftCa_SM_SelfCount).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Ganon,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Ganon),
    data_file: "PlGn.dat",
    data_symbol: "ftDataGanon",
    animation_file: "PlGnAJ.dat",
    animation_count: 318,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: KNOCKBACK_PART_ANIMATIONS,
    costumes: &[
        CostumeDescriptor {
            file: "PlGnNr.dat",
            joint_symbol: "PlyGanon5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlGnRe.dat",
            joint_symbol: "PlyGanon5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlGnBu.dat",
            joint_symbol: "PlyGanon5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlGnGr.dat",
            joint_symbol: "PlyGanon5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlGnLa.dat",
            joint_symbol: "PlyGanon5KLa_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[25]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 5;
