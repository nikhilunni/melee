//! Composition root for stage-specific resources and callbacks.
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use hsd_archive::Archive;
use melee_gr::{
    battle::Battlefield,
    desc::{ReadResult, StageDesc},
    last::{procs::ProcRegistration, AnimationStatus, FinalDestination},
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
pub fn descriptor(name: &str) -> Option<&'static StageDescriptor> {
    [&FINAL_DESTINATION, &BATTLEFIELD]
        .into_iter()
        .find(|d| d.name == name)
}
pub enum SceneStage {
    FinalDestination(Box<FinalDestination>),
    Battlefield(Battlefield),
}
impl SceneStage {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        match self {
            Self::FinalDestination(s) => s.proc_table(),
            Self::Battlefield(s) => s.proc_table(),
        }
    }
    pub fn run_stage_proc(&mut self, map: u8, rng: &mut HsdRng) -> Result<()> {
        match self {
            Self::FinalDestination(stage) => {
                stage.run_stage_proc(map, &AnimationStatus::default(), rng);
                ensure!(
                    stage.actions.is_empty(),
                    "FD transition outside restored interval: {:?}",
                    stage.actions
                );
            }
            Self::Battlefield(stage) => match map {
                3 => stage.tick(),
                // Empty callbacks; map 6 updates the static collision transform
                // and the empty quake list, guarded at restoration.
                0 | 1 | 6 => {}
                _ => unimplemented!("grbattle.c:165-170: demo/event background"),
            },
        }
        Ok(())
    }
}
