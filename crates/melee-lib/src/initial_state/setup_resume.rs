//! Resume a match save taken during costume allocation, before Fighter reset.
use super::{float, vector, word, SavedPose};
use crate::{assets::Assets, scene_fighter::SceneFighter};
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use melee_ft::fighter::{PlayerSlot, SpawnContext, SpawnCounter};

/// GetMemoryEntry (80381D58, class.c:79) just past its `idx >= 0` assert.
const GET_MEMORY_ENTRY_AFTER_ASSERT: u32 = 0x8038_1D8C;
/// Fighter_Create return addresses of the supported unfinished calls, both
/// before Fighter_UnkProcessDeath (bl at 800692E0): HSD_JObjLoadJoint (bl at
/// 80068F88) and ftAnim_8006FE48 (bl at 80069020).
const CREATE_RETURNS_BEFORE_DEATH: [u32; 2] = [0x8006_8F8C, 0x8006_9024];

/// Fighter_Create (80068E98) loads its costume before resetting gameplay
/// state and drawing the two CPU values. Rebuild that uninitialised fighter
/// through the same constructor as cold setup, using saved StaticPlayer data.
pub(super) fn fighter(
    saved: &SavedPose,
    assets: &Assets,
    slot: usize,
    map: &mut melee_mp::CollMap,
    rng: &mut HsdRng,
) -> Result<SceneFighter> {
    let (registers, pc) = saved.cpu_general_registers()?;
    // HSD_MObjAlloc, or HSD_MemAlloc just after OSAllocFromHeap (8037F210),
    // during Fighter_UnkUpdateCostumeJoint_800686E4. The latter comes from
    // HSD_SListAlloc in loadEnvelopeDesc (PObj skinning envelopes), under
    // lbRefract_PObjLoad -> HSD_DObjLoadDesc -> JObjLoad. Or GetMemoryEntry
    // just past its idx assert (80381D8C), hsdAllocMemPiece's size-class
    // lookup, while ftAnim_8006FE48 builds the animation skeleton
    // (ftParts_8007482C). All are HSD heap work inside Fighter_Create, before
    // Fighter_UnkProcessDeath, its spawn number and its CPU RNG draws.
    ensure!(
        (0x8036_3CA4..0x8036_3D00).contains(&pc)
            || pc == 0x8037_F210
            || pc == GET_MEMORY_ENTRY_AFTER_ASSERT,
        "unsupported unfinished fighter creation PC {pc:08X}"
    );
    if pc == 0x8037_F210 {
        ensure!(
            (0x8000_0000..0x8180_0000).contains(&registers[3]),
            "saved HSD allocation failed"
        );
    }
    let mut stack = registers[1];
    let mut in_create = false;
    // Recursive joint loading can place more than 70 frames above Create.
    const MAX_SETUP_STACK_FRAMES: usize = 128;
    for _ in 0..MAX_SETUP_STACK_FRAMES {
        let frame = saved.bytes(stack, 8);
        let caller = word(frame, 4);
        in_create |= CREATE_RETURNS_BEFORE_DEATH.contains(&caller);
        if in_create {
            break;
        }
        stack = word(frame, 0);
        if !(0x8000_0000..0x8180_0000).contains(&stack) {
            break;
        }
    }
    ensure!(
        in_create,
        "unfinished allocation is not inside Fighter_Create"
    );
    // pl/player.h StaticPlayer; Player_Get* assembly confirms stride 0xE90.
    let raw = saved.bytes(0x8045_3080 + slot as u32 * 0xE90, 0xB0);
    ensure!(
        raw[0xC] == 0 && word(raw, 8) == 0,
        "transformed or CPU setup is unsupported"
    );
    let player = PlayerSlot {
        id: slot as u8,
        secondary: false,
        control: melee_types::PlayerKind::Human,
        costume: raw[0x44],
        stocks: raw[0x8E],
        // Player_GetFallsByIndex(slot, 0).
        falls: u32::from_be_bytes(raw[0x68..0x6C].try_into().unwrap()),
        position: vector(raw, 0x10),
        facing: float(raw, 0x40),
        scale: float(raw, 0x5C),
        damage: f32::from(i16::from_be_bytes(raw[0x60..0x62].try_into().unwrap())),
        cpu_mode: i32::from(raw[0x4A]),
        cpu_level: i32::from(raw[0x49]),
    };
    let mut counter = SpawnCounter(word(saved.bytes(0x804D_64F8, 4), 0));
    SceneFighter::from_parameters(
        &assets.characters[slot],
        &assets.fighters[slot],
        player,
        i32::from(raw[0x4C]),
        SpawnContext {
            map,
            stage_camera: &assets.stage_camera,
            rng,
            counter: &mut counter,
        },
    )
}
