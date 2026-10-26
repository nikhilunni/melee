//! Composition root for stage-specific resources and callbacks.
pub(crate) mod battle;
pub(crate) mod fourside;
pub(crate) mod flatzone;
pub(crate) mod garden;
pub(crate) mod inishie1;
pub(crate) mod izumi;
pub(crate) mod kraid;
pub(crate) mod kongo;
pub(crate) mod last;
pub(crate) mod oldkongo;
pub(crate) mod oldyoshi;
pub(crate) mod onett;
pub(crate) mod pupupu;
pub(crate) mod shrine;
pub(crate) mod stadium;
pub(crate) mod yorster;
pub(crate) mod zebes;
use anyhow::Result;
use gekko_math::HsdRng;
use hsd_archive::Archive;
use melee_gr::{
    battle::Battlefield,
    desc::{ReadResult, StageDesc},
    last::{procs::ProcRegistration, FinalDestination},
};

/// Animation clocks advance independently. Materialize world matrices only for
/// live consumers; the fixture recorder also consumes pre-attachment history.
pub(crate) fn publish_joint_matrices(
    animation: &mut melee_gr::last::animation::BackgroundAnimation,
    map: u8,
    particles: &mut hsd_particle::system::ParticleSystem,
    events: &mut melee_ef::fixture_spawns::EventSink,
) {
    for joint in 0..animation.joint_count() {
        let id = crate::initial_state::stage::joint_id(map, joint);
        if events.needs_joint_history() || particles.has_joint_attachment(id) {
            let matrix = animation.joint_matrix(joint);
            events.update_joint(id, matrix);
            particles.update_joint(id, matrix);
        }
    }
}

/// Ground_801C2D24 (0x801C2D24) at the drop markers now: each marker is
/// its joint's world translation (lb_8000B1CC). Kongo Jungle binds some to
/// the platforms its animation moves, which the live models give; every
/// other stage's marker joints stand still, so their positions at setup
/// hold (each stage's `w26_start_*_items_all` boundary witnesses its drops).
/// Pokemon Stadium's forms bind no markers (GrPs1..4.dat's map_head section
/// 0 is empty for maps 3, 4 and 6..9), so its stay too.
pub(crate) fn live_drop_markers(
    assets: &crate::assets::Assets,
    animations: &mut std::collections::BTreeMap<u8, melee_gr::last::animation::BackgroundAnimation>,
) -> melee_gr::bomb_rain::DropMarkers {
    let mut markers = assets.drop_markers.clone();
    if assets.stage_desc.kind != melee_types::GrKind::Kongo {
        return markers;
    }
    let joints = &assets.drop_marker_joints;
    let slots = markers.items.iter_mut().zip(&joints.items);
    for (slot, binding) in slots.chain(markers.spawns.iter_mut().zip(&joints.spawns)) {
        let (Some(_), Some((model, joint))) = (*slot, *binding) else {
            continue;
        };
        if let Some(animation) = u8::try_from(model).ok().and_then(|m| animations.get_mut(&m)) {
            let m = animation.joint_matrix(joint);
            *slot = Some(hsd_types::Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3]));
        }
    }
    markers
}

/// Live Ground GObjs by scheduler key: map ids, plus Fountain of Dreams'
/// star and second platform (melee_gr::izumi::procs).
pub(crate) type StageObjects = [Option<hsd_gobj::GObjId>; 16];

/// The addresses of the map GObjs a stage created during the match
/// (Ground_GetStageGObj, 0x801C14D0: GObj_Create(HSD_GOBJ_CLASS_STAGE, 5,
/// 3)), oldest first. One the match began with has an address no object
/// of the match has had.
#[derive(Clone, Default)]
pub(crate) struct StageGObjs {
    created: melee_types::fixed::FixedVec<(u8, hsd_gobj::GObjAddress), 16>,
}

impl StageGObjs {
    /// Ground_GetStageGObj for `map`.
    pub(crate) fn create(&mut self, gobjs: &mut hsd_gobj::GObjAddresses, map: u8) {
        self.created
            .push((map, gobjs.create(hsd_gobj::GObjClass::Stage)));
    }
    /// Ground_801C4A08 (0x801C4A08) on `map`'s oldest GObj.
    pub(crate) fn destroy(&mut self, gobjs: &mut hsd_gobj::GObjAddresses, map: u8) {
        self.destroy_nth(gobjs, map, 0);
    }
    /// Ground_801C4A08 on the `nth` oldest of `map`'s GObjs.
    pub(crate) fn destroy_nth(
        &mut self,
        gobjs: &mut hsd_gobj::GObjAddresses,
        map: u8,
        nth: usize,
    ) {
        let mut index = None;
        let mut seen = 0;
        for (at, &(created, _)) in self.created.iter().enumerate() {
            if created == map {
                if seen == nth {
                    index = Some(at);
                    break;
                }
                seen += 1;
            }
        }
        let address = match index {
            Some(index) => self.created.remove(index).1,
            None => gobjs.outside(),
        };
        gobjs.destroy(hsd_gobj::GObjClass::Stage, address);
    }
}

pub struct StageDescriptor {
    pub name: &'static str,
    pub file: &'static str,
    pub music_id: i32,
    /// The Ground kind the stage's creation runs (fixes its setup RNG draws).
    pub kind: melee_types::GrKind,
    pub read: fn(&Archive) -> ReadResult<StageDesc>,
}
pub const FINAL_DESTINATION: StageDescriptor = StageDescriptor {
    name: "FinalDestination",
    file: "GrNLa.dat",
    music_id: 32,
    kind: melee_types::GrKind::Last,
    read: melee_gr::desc::read_final_destination,
};
pub const BATTLEFIELD: StageDescriptor = StageDescriptor {
    name: "Battlefield",
    file: "GrNBa.dat",
    music_id: 31,
    kind: melee_types::GrKind::Battle,
    read: melee_gr::desc::read_battlefield,
};
pub const YOSHIS_STORY: StageDescriptor = StageDescriptor {
    name: "YoshisStory",
    file: "GrSt.dat",
    music_id: 8,
    kind: melee_types::GrKind::Story,
    read: melee_gr::desc::read_story,
};
pub const DREAM_LAND: StageDescriptor = StageDescriptor {
    name: "DreamLand",
    file: "GrOp.dat",
    music_id: 28,
    kind: melee_types::GrKind::OldPupupu,
    read: melee_gr::desc::read_pupupu,
};
/// grIz_StageData (0x803E0E5C): GrIz.dat; StageParam row 2 (StKind).
pub const FOUNTAIN_OF_DREAMS: StageDescriptor = StageDescriptor {
    name: "FountainOfDreams",
    file: "GrIz.dat",
    music_id: 2,
    kind: melee_types::GrKind::Izumi,
    read: melee_gr::desc::read_izumi,
};
pub const POKEMON_STADIUM: StageDescriptor = StageDescriptor {
    name: "PokemonStadium",
    file: "GrPs.dat",
    music_id: 3,
    kind: melee_types::GrKind::PStadium,
    read: melee_gr::desc::read_stadium,
};
/// grOy_StageData: GrOy.dat; StageParam row 0x1D (StKind).
pub const YOSHIS_ISLAND_64: StageDescriptor = StageDescriptor {
    name: "YoshisIsland64",
    file: "GrOy.dat",
    music_id: 0x1D,
    kind: melee_types::GrKind::OldYoshi,
    read: melee_gr::desc::read_old_yoshi,
};
/// grOk_StageData: GrOk.dat; StageParam row 0x1E (StKind).
pub const KONGO_JUNGLE_64: StageDescriptor = StageDescriptor {
    name: "KongoJungle64",
    file: "GrOk.dat",
    music_id: 0x1E,
    kind: melee_types::GrKind::OldKongo,
    read: melee_gr::desc::read_old_kongo,
};
/// grGd_StageData (0x803E52E0): GrGd.dat; StageParam row 12 (StKind).
pub const JUNGLE_JAPES: StageDescriptor = StageDescriptor {
    name: "JungleJapes",
    file: "GrGd.dat",
    music_id: 12,
    kind: melee_types::GrKind::Garden,
    read: melee_gr::desc::read_garden,
};
/// grKg_StageData (0x803E1800): GrKg.dat; StageParam row 5 (StKind).
pub const KONGO_JUNGLE: StageDescriptor = StageDescriptor {
    name: "KongoJungle",
    file: "GrKg.dat",
    music_id: 5,
    kind: melee_types::GrKind::Kongo,
    read: melee_gr::desc::read_kongo,
};
/// grSh_StageData: GrSh.dat; StageParam row 0x0E (StKind).
pub const HYRULE_TEMPLE: StageDescriptor = StageDescriptor {
    name: "HyruleTemple",
    file: "GrSh.dat",
    music_id: 0x0E,
    kind: melee_types::GrKind::Shrine,
    read: melee_gr::desc::read_shrine,
};
/// grFs_StageData: GrFs.dat; StageParam row 0x12 (StKind).
pub const FOURSIDE: StageDescriptor = StageDescriptor {
    name: "Fourside",
    file: "GrFs.dat",
    music_id: 0x12,
    kind: melee_types::GrKind::Fourside,
    read: melee_gr::desc::read_fourside,
};
/// grKr_StageData: GrKr.dat; StageParam row 0x0F (StKind).
pub const BRINSTAR_DEPTHS: StageDescriptor = StageDescriptor {
    name: "BrinstarDepths",
    file: "GrKr.dat",
    music_id: 0x0F,
    kind: melee_types::GrKind::Kraid,
    read: melee_gr::desc::read_kraid,
};
/// grFz_StageData: GrFz.dat; StageParam row 0x1B (StKind).
pub const FLAT_ZONE: StageDescriptor = StageDescriptor {
    name: "FlatZone",
    file: "GrFz.dat",
    music_id: 0x1B,
    kind: melee_types::GrKind::Flatzone,
    read: melee_gr::desc::read_flatzone,
};
/// grYt_StageData: GrYt.dat; StageParam row 0x10 (StKind).
pub const YOSHIS_ISLAND: StageDescriptor = StageDescriptor {
    name: "YoshisIsland",
    file: "GrYt.dat",
    music_id: 0x10,
    kind: melee_types::GrKind::Yorster,
    read: melee_gr::desc::read_yorster,
};
/// grI1_StageData: GrI1.dat; StageParam row 0x13 (StKind).
pub const MUSHROOM_KINGDOM: StageDescriptor = StageDescriptor {
    name: "MushroomKingdom",
    file: "GrI1.dat",
    music_id: 0x13,
    kind: melee_types::GrKind::Inishie1,
    read: melee_gr::desc::read_inishie1,
};
/// grOt_StageData: GrOt.dat; StageParam row 0x09 (StKind).
pub const ONETT: StageDescriptor = StageDescriptor {
    name: "Onett",
    file: "GrOt.dat",
    music_id: 0x09,
    kind: melee_types::GrKind::Onett,
    read: melee_gr::desc::read_onett,
};
/// grZe_StageData: GrZe.dat; StageParam row 0x06 (StKind).
pub const BRINSTAR: StageDescriptor = StageDescriptor {
    name: "Brinstar",
    file: "GrZe.dat",
    music_id: 0x06,
    kind: melee_types::GrKind::Zebes,
    read: melee_gr::desc::read_zebes,
};
pub fn descriptor(name: &str) -> Option<&'static StageDescriptor> {
    [
        &FINAL_DESTINATION,
        &BATTLEFIELD,
        &YOSHIS_STORY,
        &DREAM_LAND,
        &FOUNTAIN_OF_DREAMS,
        &POKEMON_STADIUM,
        &YOSHIS_ISLAND_64,
        &KONGO_JUNGLE_64,
        &JUNGLE_JAPES,
        &KONGO_JUNGLE,
        &HYRULE_TEMPLE,
        &FOURSIDE,
        &BRINSTAR_DEPTHS,
        &FLAT_ZONE,
        &YOSHIS_ISLAND,
        &MUSHROOM_KINGDOM,
        &ONETT,
        &BRINSTAR,
    ]
    .into_iter()
    .find(|d| d.name == name)
}
#[derive(Clone)]
pub enum SceneStage {
    FinalDestination(Box<FinalDestination>),
    Battlefield(Battlefield),
    Story(melee_gr::story::Story),
    Pupupu(melee_gr::pupupu::Pupupu),
    Izumi(melee_gr::izumi::Izumi),
    Stadium(Box<melee_gr::stadium::Stadium>),
    OldYoshi(melee_gr::oldyoshi::OldYoshi),
    OldKongo(melee_gr::oldkongo::OldKongo),
    Garden(melee_gr::garden::Garden),
    Kongo(Box<melee_gr::kongo::Kongo>),
    Shrine(melee_gr::shrine::Shrine),
    Fourside(Box<melee_gr::fourside::Fourside>),
    Kraid(Box<melee_gr::kraid::procs::KraidStage>),
    FlatZone(Box<melee_gr::flatzone::FlatZone>),
    Yorster(Box<melee_gr::yorster::procs::YorsterStage>),
    Inishie1(Box<melee_gr::inishie1::procs::Inishie1Stage>),
    Onett(Box<melee_gr::onett::procs::OnettStage>),
    Zebes(Box<melee_gr::zebes::procs::ZebesStage>),
}
impl SceneStage {
    /// Install the frozen-stage code (`SlippiCodes::frozen_stages`) on the
    /// stage's own site. Battlefield and Fountain of Dreams have none.
    pub fn set_frozen_stages(&mut self, frozen: bool) {
        match self {
            Self::FinalDestination(s) => s.ground.phase_update_disabled = frozen,
            Self::Story(s) => s.shy_guys_disabled = frozen,
            Self::Pupupu(s) => s.wind_disabled = frozen,
            Self::Stadium(s) => s.transformation.disabled = frozen,
            Self::Battlefield(_) | Self::Izumi(_) | Self::OldYoshi(_) | Self::OldKongo(_) | Self::Garden(_) | Self::Kongo(_) | Self::Shrine(_) | Self::Fourside(_) | Self::Kraid(_) | Self::FlatZone(_) | Self::Yorster(_) | Self::Inishie1(_) | Self::Onett(_) | Self::Zebes(_) => {}
        }
    }
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        match self {
            Self::FinalDestination(s) => s.proc_table(),
            Self::Battlefield(s) => s.proc_table(),
            Self::Story(s) => s.proc_table(),
            Self::Pupupu(s) => s.proc_table(),
            Self::Izumi(s) => s.proc_table(),
            Self::Stadium(s) => s.proc_table(),
            Self::OldYoshi(s) => s.proc_table(),
            Self::OldKongo(s) => s.proc_table(),
            Self::Garden(s) => s.proc_table(),
            Self::Kongo(s) => s.proc_table(),
            Self::Shrine(s) => s.proc_table(),
            Self::Fourside(s) => s.proc_table(),
            Self::Kraid(s) => s.proc_table(),
            Self::FlatZone(s) => s.proc_table(),
            Self::Yorster(s) => s.proc_table(),
            Self::Inishie1(s) => s.proc_table(),
            Self::Onett(s) => s.proc_table(),
            Self::Zebes(s) => s.proc_table(),
        }
    }
    /// Procs whose GObjs the stage creates when the countdown releases it.
    /// The GObjs Ground_801C0FB8 (ground.c:694-709) creates when the
    /// countdown ends, in order: the stage's on_start (each tournament
    /// stage's is grZakoGenerator_801CAE04 alone), then the GObj of
    /// Ground_801C0C2C's proc.
    pub fn start_gobjs(&self) -> &'static [hsd_gobj::GObjClass; 2] {
        &[
            hsd_gobj::GObjClass::EnemyGenerator,
            hsd_gobj::GObjClass::Stage,
        ]
    }
    pub fn start_proc_table(&self) -> Vec<ProcRegistration> {
        match self {
            Self::FinalDestination(s) => s.start_proc_table(),
            _ => Vec::new(),
        }
    }
    pub fn run_stage_proc(&mut self, map: u8, rng: &mut HsdRng) -> Result<bool> {
        match self {
            Self::Pupupu(_) => unreachable!("Dream Land callbacks require animation state"),
            Self::Izumi(_) => unreachable!("Fountain of Dreams callbacks require animation state"),
            Self::Stadium(_) => unreachable!("Pokemon Stadium callbacks require animation state"),
            Self::OldYoshi(_) => {
                unreachable!("Yoshi's Island N64 callbacks require animation state")
            }
            Self::OldKongo(_) => {
                unreachable!("Kongo Jungle N64 callbacks require animation state")
            }
            Self::Garden(_) => unreachable!("Jungle Japes callbacks require animation state"),
            Self::Kongo(_) => unreachable!("Kongo Jungle callbacks require animation state"),
            Self::Shrine(_) => unreachable!("Hyrule Temple callbacks run in scene_stage::shrine"),
            Self::Fourside(_) => unreachable!("Fourside callbacks run in scene_stage::fourside"),
            Self::Kraid(_) => unreachable!("Brinstar Depths callbacks run in scene_stage::kraid"),
            Self::FlatZone(_) => unreachable!("Flat Zone callbacks require animation state"),
            Self::Yorster(_) => unreachable!("Yoshi's Island callbacks run in scene_stage::yorster"),
            Self::Inishie1(_) => unreachable!("Mushroom Kingdom callbacks run in scene_stage::inishie1"),
            Self::Onett(_) => unreachable!("Onett callbacks run in scene_stage::onett"),
            Self::Zebes(_) => unreachable!("Brinstar callbacks run in scene_stage::zebes"),
            Self::FinalDestination(_) => unreachable!("FD callbacks require animation state"),
            Self::Story(stage) => match map {
                1 => {}
                3 => unreachable!("the Shy Guy spawner needs the item pool"),
                2 => return Ok(stage.tick_puff(rng)),
                _ => unreachable!("Story callback map"),
            },
            Self::Battlefield(_) => {
                unreachable!("Battlefield callbacks require animation state")
            }
        }
        Ok(false)
    }
}

/// The stage's camera limits as the camera reads them this tick: Brinstar's
/// floor (stage_info.x724, Ground_801C4368) follows the acid, and so does
/// Camera_8002AF68's lowest eye height (grZebes_801DCCC8: 5.0 + the
/// surface, fadds).
pub(crate) fn live_camera(stage: &SceneStage, assets: &crate::assets::Assets) -> melee_cm::StageCamera {
    let mut camera = assets.stage_camera;
    if let SceneStage::Zebes(stage) = stage {
        camera.floor = stage.floor;
        camera.min_eye_height = 5.0 + stage.floor;
    }
    camera
}

/// The stage callbacks a collider's proc made on joint ceilings
/// (`mpJointSetCb2`): Yoshi's Island's blocks and Mushroom Kingdom's
/// bricks apply them once the proc returns.
pub(crate) fn apply_ceiling_events(
    state: &mut crate::initial_state::InitialState,
    collider: yorster::Collider,
) -> Result<()> {
    match state.stage {
        SceneStage::Yorster(_) => yorster::apply_bumps(state, collider),
        SceneStage::Inishie1(_) => inishie1::apply_bumps(state),
        _ => Ok(()),
    }
}

/// A stage material item's hit-taken callback (yaku.x18).
pub(crate) fn material_hit(
    state: &mut crate::initial_state::InitialState,
    item: u32,
    fighter: Option<usize>,
    damage: i32,
    launch_x: f32,
) -> Result<()> {
    match state.stage {
        SceneStage::Zebes(_) => {
            zebes::material_hit(state, item, damage, launch_x);
            Ok(())
        }
        SceneStage::Yorster(_) => {
            yorster::material_hit(state, item, fighter, damage);
            Ok(())
        }
        SceneStage::Inishie1(_) => inishie1::material_hit(state, item),
        _ => Ok(()),
    }
}

/// A stage material item's damage-dealt callback (yaku.x14).
pub(crate) fn material_dealt(state: &mut crate::initial_state::InitialState, item: u32) {
    if let SceneStage::Onett(_) = state.stage {
        onett::car_struck(state, item);
    }
}
