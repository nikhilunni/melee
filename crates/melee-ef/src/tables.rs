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
pub(super) static MODEL_SPAWNS: [ModelSpawn; 16] = [
    // S6: color-overlay landing dust, same efAsync row as the landing opcode.
    ModelSpawn {
        request: 0x404,
        source: ModelSource::Graphics,
        model: 0x18,
        attached: false,
    },
    // S3: Counter script dust, efasync.c:221-227.
    ModelSpawn {
        request: 0x3F9,
        source: ModelSource::Graphics,
        model: 0x14,
        attached: false,
    },
    // S3/S7: efAsync_Dispatch 0x3FA, Reflector's script light and the positional throw
    // flash (no orientation arguments); both lanes ported the same retail site.
    ModelSpawn {
        request: 0x3FA,
        source: ModelSource::Graphics,
        model: 0x15,
        attached: false,
    },
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
    // efAsync_Dispatch 8006528C: successful powershield model.
    ModelSpawn {
        request: 0x41A,
        source: ModelSource::Shield,
        model: 0xE,
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
pub(super) static DUST_SPAWNS: [DustSpawn; 13] = [
    // efasync.c 0x410 (80063... efLib_CreateGenerator 0x22A): an item's
    // explosion (it_80272C08).
    DustSpawn {
        request: 0x410,
        particle: 0x22A,
        directional: false,
    },
    // S6: dizzy animation sparkle, efasync.c:117-119.
    DustSpawn {
        request: 0x3E9,
        particle: 0xC,
        directional: false,
    },
    // efAsync_Dispatch 80063AEC..B08: Fire hit spark, generator 0x14.
    DustSpawn {
        request: 0x3EA,
        particle: 0x14,
        directional: false,
    },
    // S9: efasync.c 0x42D, the star-KO twinkle (efLib_CreateGenerator 0x121 at cur_pos).
    DustSpawn {
        request: 0x42D,
        particle: 0x121,
        directional: false,
    },
    // S3: efasync.c:174-185, Dolphin Slash launch dust.
    DustSpawn {
        request: 0x3F1,
        particle: 0x14B,
        directional: true,
    },
    DustSpawn {
        request: 0x3F2,
        particle: 0x14B,
        directional: true,
    },
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
// efAsync_Dispatch80064E50..64: Fire body overlay uses attached generator0x37.
pub(super) static ATTACHED_SPAWNS: [(u16, u32); 5] = [
    (0x402, 0x59),
    (0x403, 0x5E),
    (0x412, 0x13),
    (0x413, 0x37),
    (0x414, 0xE1),
];
// efLib_SpawnParticleEffect (8005D174), ordinary supported DPtcl outputs.
pub(super) static PARTICLE_KINDS: [i32; 23] = [
    2, 6, 8, 9, 10, 45, 46, 212, 261, 266, 267, 306, 307, 364, 365, 366, 367, 368, 372, 373, 445,
    448, 449,
];
