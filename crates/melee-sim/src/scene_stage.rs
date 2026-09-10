//! Composition root for stage-specific resources and callbacks.
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
pub fn descriptor(name: &str) -> Option<&'static StageDescriptor> {
    [&FINAL_DESTINATION, &BATTLEFIELD, &YOSHIS_STORY, &DREAM_LAND]
        .into_iter()
        .find(|d| d.name == name)
}
pub enum SceneStage {
    FinalDestination(Box<FinalDestination>),
    Battlefield(Battlefield),
    Story(melee_gr::story::Story),
    Pupupu(melee_gr::pupupu::Pupupu),
}
impl SceneStage {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        match self {
            Self::FinalDestination(s) => s.proc_table(),
            Self::Battlefield(s) => s.proc_table(),
            Self::Story(s) => s.proc_table(),
            Self::Pupupu(s) => s.proc_table(),
        }
    }
    pub fn run_stage_proc(&mut self, map: u8, rng: &mut HsdRng) -> Result<bool> {
        match self {
            Self::Pupupu(_) => unreachable!("Dream Land callbacks require animation state"),
            Self::FinalDestination(_) => unreachable!("FD callbacks require animation state"),
            Self::Story(stage) => match map {
                1 => {}
                3 => stage.tick_shy_guys(rng),
                2 => return Ok(stage.tick_puff(rng)),
                _ => unreachable!("Story callback map"),
            },
            Self::Battlefield(stage) => match map {
                3 => stage.tick(),
                // Empty callbacks; map 6 updates the static collision transform
                // and the empty quake list, guarded at restoration.
                0 | 1 | 6 => {}
                _ => unimplemented!("grbattle.c:165-170: demo/event background"),
            },
        }
        Ok(false)
    }
}
