//! The supported efAsync_Dispatch / efLib_SpawnParticleEffect table rows.
//! Numeric data is retail dispatch data, never a recording-derived schedule.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelSource {
    Graphics,
    Shield,
    Landing,
    Entry,
}
#[derive(Clone, Copy)]
pub(super) struct ModelSpawn {
    pub request: u16,
    pub source: ModelSource,
    pub model: u32,
    pub attached: bool,
}
// efasync.c:205-212,262-293,750-756; efsync.c shield dispatch.
pub(super) static MODEL_SPAWNS: [ModelSpawn; 12] = [
    ModelSpawn {
        request: 0x3FB,
        source: ModelSource::Graphics,
        model: 0x16,
        attached: false,
    },
    ModelSpawn {
        request: 0x423,
        source: ModelSource::Graphics,
        model: 1,
        attached: true,
    },
    ModelSpawn {
        request: 0x3FD,
        source: ModelSource::Graphics,
        model: 3,
        attached: false,
    },
    ModelSpawn {
        request: 0x424,
        source: ModelSource::Graphics,
        model: 2,
        attached: true,
    },
    ModelSpawn {
        request: 0x3F7,
        source: ModelSource::Graphics,
        model: 0x12,
        attached: false,
    },
    ModelSpawn {
        request: 0x3F8,
        source: ModelSource::Graphics,
        model: 0x13,
        attached: false,
    },
    ModelSpawn {
        request: 0x3FF,
        source: ModelSource::Graphics,
        model: 5,
        attached: false,
    },
    ModelSpawn {
        request: 0x404,
        source: ModelSource::Landing,
        model: 0x18,
        attached: false,
    },
    ModelSpawn {
        request: 0x406,
        source: ModelSource::Graphics,
        model: 4,
        attached: false,
    },
    ModelSpawn {
        request: 0x417,
        source: ModelSource::Shield,
        model: 0xB,
        attached: true,
    },
    ModelSpawn {
        request: 0x418,
        source: ModelSource::Shield,
        model: 0xC,
        attached: true,
    },
    ModelSpawn {
        request: 0x419,
        source: ModelSource::Shield,
        model: 0xD,
        attached: true,
    },
];
pub(super) static WARP_SPAWN: ModelSpawn = ModelSpawn {
    request: 0x43E,
    source: ModelSource::Entry,
    model: 0x24,
    attached: true,
};

#[derive(Clone, Copy)]
pub(super) struct DustSpawn {
    pub request: u16,
    pub particle: u32,
    pub directional: bool,
}
// efasync.c:186-188,255-282,305-307,521-523.
pub(super) static DUST_SPAWNS: [DustSpawn; 7] = [
    DustSpawn {
        request: 0x3EF,
        particle: 0x42,
        directional: true,
    },
    DustSpawn {
        request: 0x3F3,
        particle: 0xB,
        directional: false,
    },
    DustSpawn {
        request: 0x3FE,
        particle: 0x107,
        directional: true,
    },
    DustSpawn {
        request: 0x400,
        particle: 0x5A,
        directional: true,
    },
    DustSpawn {
        request: 0x401,
        particle: 0x5A,
        directional: true,
    },
    DustSpawn {
        request: 0x407,
        particle: 0x3C,
        directional: false,
    },
    DustSpawn {
        request: 0x41C,
        particle: 0x5D,
        directional: false,
    },
];
// efasync.c:282-287, live-joint generator dispatch.
pub(super) static ATTACHED_SPAWNS: [(u16, u32); 2] = [(0x402, 0x59), (0x403, 0x5E)];
// efLib_SpawnParticleEffect (8005D174), ordinary supported DPtcl outputs.
pub(super) static PARTICLE_KINDS: [i32; 17] = [
    2, 6, 8, 9, 10, 45, 212, 261, 266, 267, 306, 307, 364, 373, 445, 448, 449,
];
