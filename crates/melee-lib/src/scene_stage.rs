//! Composition root for stage-specific resources and callbacks.
pub(crate) mod battle;
pub(crate) mod izumi;
pub(crate) mod last;
pub(crate) mod pupupu;
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

/// Live Ground GObjs by scheduler key: map ids, plus Fountain of Dreams'
/// star and second platform (melee_gr::izumi::procs).
pub(crate) type StageObjects = [Option<hsd_gobj::GObjId>; 16];

pub struct StageDescriptor {
    pub name: &'static str,
    pub file: &'static str,
    pub music_id: i32,
    pub read: fn(&Archive) -> ReadResult<StageDesc>,
}
pub const FINAL_DESTINATION: StageDescriptor = StageDescriptor {
    name: "FinalDestination",
    file: "GrNLa.dat",
    music_id: 32,
    read: melee_gr::desc::read_final_destination,
};
pub const BATTLEFIELD: StageDescriptor = StageDescriptor {
    name: "Battlefield",
    file: "GrNBa.dat",
    music_id: 31,
    read: melee_gr::desc::read_battlefield,
};
pub const YOSHIS_STORY: StageDescriptor = StageDescriptor {
    name: "YoshisStory",
    file: "GrSt.dat",
    music_id: 8,
    read: melee_gr::desc::read_story,
};
pub const DREAM_LAND: StageDescriptor = StageDescriptor {
    name: "DreamLand",
    file: "GrOp.dat",
    music_id: 28,
    read: melee_gr::desc::read_pupupu,
};
/// grIz_StageData (0x803E0E5C): GrIz.dat; StageParam row 2 (StKind).
pub const FOUNTAIN_OF_DREAMS: StageDescriptor = StageDescriptor {
    name: "FountainOfDreams",
    file: "GrIz.dat",
    music_id: 2,
    read: melee_gr::desc::read_izumi,
};
pub fn descriptor(name: &str) -> Option<&'static StageDescriptor> {
    [
        &FINAL_DESTINATION,
        &BATTLEFIELD,
        &YOSHIS_STORY,
        &DREAM_LAND,
        &FOUNTAIN_OF_DREAMS,
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
}
impl SceneStage {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        match self {
            Self::FinalDestination(s) => s.proc_table(),
            Self::Battlefield(s) => s.proc_table(),
            Self::Story(s) => s.proc_table(),
            Self::Pupupu(s) => s.proc_table(),
            Self::Izumi(s) => s.proc_table(),
        }
    }
    /// Procs whose GObjs the stage creates when the countdown releases it.
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
