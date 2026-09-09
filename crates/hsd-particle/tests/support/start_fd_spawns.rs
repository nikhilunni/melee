//! Effect inputs for start_fd_fox; transforms are supplied from
//! effect-joint captures. See START_FD.md for the explicit input boundary.
use crate::common::RetailTrig;
use gekko_math::rng::HsdRng;
use hsd_archive::Archive;
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Mtx;
use std::collections::BTreeMap;

pub fn common_bank(archive: &Archive) -> ParticleBank {
    // efAsync_LoadSync (efasync.c:1287-1316): effCommonDataTable begins with command/texture pointers.
    let table = archive.public("effCommonDataTable").unwrap();
    let commands = archive.link(table).unwrap().unwrap() as usize;
    let textures = archive.link(table + 4).unwrap().unwrap() as usize;
    ParticleBank::from_bytes(
        &archive.data()[commands..textures],
        &archive.data()[textures..],
    )
    .unwrap()
}

/// The external creation schedule. The 160 EF children are deliberately absent:
/// their requests and allocator increments must come from interpreter execution.
const REQUESTS: [(usize, u8, u32, usize); 9] = [
    (1, 30, 30000, 0),
    (6, 0, 445, 1),
    (6, 0, 449, 2),
    (11, 0, 445, 3),
    (11, 0, 449, 4),
    (15, 0, 448, 2),
    (20, 0, 448, 4),
    (75, 0, 10, 5),
    (80, 0, 10, 6),
];

pub struct Spawns {
    matrices: BTreeMap<usize, Mtx>,
    changes: Vec<(usize, usize, [u32; 12])>,
}
impl Spawns {
    pub fn new() -> Self {
        // Only effect-owned cached joint matrices enter this fixture. Particle
        // lists, counters, RNG, emitted positions/velocities never enter it.
        Self {
            matrices: BTreeMap::new(),
            changes: serde_json::from_str(include_str!("start_fd_joints.json")).unwrap(),
        }
    }
    // Shared by several test binaries; not every one calls this method.
    #[allow(dead_code)]
    pub fn after_particles(
        &mut self,
        _tick: usize,
        _system: &mut ParticleSystem,
        _banks: &BTreeMap<u8, ParticleBank>,
        _rng: &mut HsdRng,
        _draws: &mut DrawLog,
    ) {
    }
    pub fn before_main(
        &mut self,
        tick: usize,
        system: &mut ParticleSystem,
        banks: &BTreeMap<u8, ParticleBank>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) {
        for &(frame, joint, words) in &self.changes {
            if frame == tick {
                let matrix = Mtx(std::array::from_fn(|row| {
                    std::array::from_fn(|col| f32::from_bits(words[row * 4 + col]))
                }));
                self.matrices.insert(joint, matrix);
                system.update_joint(joint, matrix);
            }
        }
        for &(frame, bank, kind, joint) in &REQUESTS {
            if frame == tick {
                let mut request = SpawnRequest::new(bank, kind, 0);
                request.joint = Some((joint, self.matrices[&joint]));
                system
                    .spawn::<RetailTrig>(&banks[&bank], request, rng, draws)
                    .unwrap();
            }
        }
    }
}
