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
pub(super) static MODEL_SPAWNS: [ModelSpawn; 24] = [
    // efasync.c:192-197: model 0x10 at a position, facing only (kind 5).
    ModelSpawn {
        request: 0x3F5,
        source: ModelSource::Graphics,
        model: 0x10,
        attached: false,
    },
    // efasync.c:199-204: 0x3F6, model 0x11 at a point, no orientation
    // (Yoshi's egg breaking on the ground).
    ModelSpawn {
        request: 0x3F6,
        source: ModelSource::Graphics,
        model: 0x11,
        attached: false,
    },
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
    // efAsync_Dispatch 0x3FC: efLib_Create_Attach_Pos(0x17), like 0x3FB.
    ModelSpawn {
        request: 0x3FC,
        source: ModelSource::Graphics,
        model: 0x17,
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
    // efAsync kind 2 0x41D (efasync.c:524-529): efLib_Create_Attach_Pos(0xF)
    // at the bone-offset point, like the wall jump's flash.
    ModelSpawn {
        request: 0x41D,
        source: ModelSource::Graphics,
        model: 0xF,
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
    // efsync.c:118-163: Pikachu's script models (efAsync kind 3 with the
    // floor angle), efLib_Create_Attach_Scale_FacingDir then rotation Z.
    ModelSpawn {
        request: 0x4C1,
        source: ModelSource::Graphics,
        model: 0x1B59,
        attached: true,
    },
    ModelSpawn {
        request: 0x4C2,
        source: ModelSource::Graphics,
        model: 0x1B5A,
        attached: true,
    },
    ModelSpawn {
        request: 0x4C4,
        source: ModelSource::Graphics,
        model: 0x1B5B,
        attached: true,
    },
    ModelSpawn {
        request: 0x4C5,
        source: ModelSource::Graphics,
        model: 0x1B5C,
        attached: true,
    },
];
/// efSync's efLib_Create_Attach_Scale_FacingDir rows among `MODEL_SPAWNS`.
pub(super) const fn scaled_facing_graphics(request: u16) -> bool {
    matches!(request, 0x4C1 | 0x4C2 | 0x4C4 | 0x4C5)
}
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
pub(super) static DUST_SPAWNS: [DustSpawn; 21] = [
    // efasync.c:353-355: efLib_CreateGenerator(0x19) at the point (it_80272AC4,
    // an item meeting the floor: Thunder's lead bolt).
    DustSpawn {
        request: 0x40C,
        particle: 0x19,
        directional: false,
    },
    // efasync.c:381-383: an article's vanishing puff (it_80272BA4) and
    // Mario's forward smash script, efLib_CreateGenerator 0x4B.
    DustSpawn {
        request: 0x411,
        particle: 0x4B,
        directional: false,
    },
    // efasync.c 0x410 (80063... efLib_CreateGenerator 0x22A): an item's
    // explosion (it_80272C08).
    DustSpawn {
        request: 0x410,
        particle: 0x22A,
        directional: false,
    },
    // efasync.c:295-297: an item's wall or ceiling bounce (it_80277C40).
    DustSpawn {
        request: 0x405,
        particle: 0x2C,
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
    // efasync.c:162-172: 0x3F0 is 0x3EF's generator facing the other way.
    DustSpawn {
        request: 0x3F0,
        particle: 0x42,
        directional: true,
    },
    DustSpawn {
        request: 0x3F3,
        particle: 0xB,
        directional: false,
    },
    // efasync.c:189-191: efLib_CreateGenerator(0x48) at the offset position.
    DustSpawn {
        request: 0x3F4,
        particle: 0x48,
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
    // efasync.c:539-541: an item's destroy effect (ItemAttr x64/x68).
    DustSpawn {
        request: 0x421,
        particle: 0x3F,
        directional: false,
    },
    // efasync.c:372-374: efLib_CreateGenerator(0x43) at the offset position
    // (Falcon Dive's catch script).
    DustSpawn {
        request: 0x40E,
        particle: 0x43,
        directional: false,
    },
    // efalt.c:35-37: efAlt_Spawn 0x479, Mario's bank-1 generator 0x3F2 at a
    // position (a coin hit, ftColl hit_effect_ids[HitElement_Coin]).
    DustSpawn {
        request: 0x479,
        particle: 0x3F2,
        directional: false,
    },
];
// efsync.c efSync_Spawn rows that call efLib_Create_Attach_Pos: the
// request id and the model it creates at the given point.
pub(super) static POSITIONAL_MODELS: [(u16, u32); 1] = [
    // efsync.c:433-435: Peach's vegetable pull.
    (0x4D2, 0x3A98),
];
// efasync.c:282-287, live-joint generator dispatch.
// efAsync_Dispatch80064E50..64: Fire body overlay uses attached generator0x37.
pub(super) static ATTACHED_SPAWNS: [(u16, u32); 8] = [
    (0x402, 0x59),
    (0x403, 0x5E),
    (0x412, 0x13),
    (0x413, 0x37),
    (0x414, 0xE1),
    // efasync.c:542-544: the item pickup sparkle.
    (0x422, 0x5B),
    // efsync.c:443-445: Peach's float sparkle, efLib_CreateGenerator_Attach.
    (0x4D4, 0x11E),
    // efsync.c:430-432: the vegetable pull's script effect, hsd_8039EFAC.
    (0x4D1, 0x64),
];
// eflib.c:761-768: efLib_CreateGenerator_Attach clears PSAPPSRT_UNK_B10
// after hsd_8039EFAC; the other attached rows keep it.
pub(super) static ATTACHED_CLEARS_B10: [u16; 1] = [0x4D4];
// efLib_SpawnParticleEffect (8005D174), ordinary supported DPtcl outputs.
pub(super) static PARTICLE_KINDS: [i32; 31] = [
    2, 6, 8, 9, 10, 45, 46, 212, 261, 266, 267, 290, 306, 307, 364, 365, 366, 367, 368, 372, 373,
    374, 375, 376, 377, 445, 448, 449, 272, 295, 531,
];
