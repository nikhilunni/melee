//! Direct grlast.c state machine. Engine dependencies are explicit actions and
//! observations; the pending particle/material interpreters must not be confused
//! with direct stage RNG draws. See docs/FD_STAGE.md.
pub mod animation;
pub mod background;
pub mod init;
pub mod lights;
pub mod procs;

use crate::ground::Ground;

/// Animation/engine operations issued in C order. The engine must apply them
/// immediately (in particular same-tick creation/deletion), before the next proc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageAction {
    CreateMap(u8),
    DestroyMap(u8),
    PlayAnimation {
        map: u8,
        joint: u16,
        animation: u8,
        subtree: bool,
    },
    StopAnimation {
        map: u8,
        joint: u16,
    },
    RequestAnimation {
        map: u8,
        joint: u16,
    },
    RemoveJointGenerators {
        map: u8,
        joint: u16,
    },
    CreateTiltGenerator,
    RemoveTiltGenerator,
    UpdateTiltGeneratorTransform,
    MaterialFade {
        map: u8,
        script: usize,
    },
    Quake,
}

/// Inputs read by grLast_8021B5C4 from animation and material runtimes, after
/// s_link 1. They cannot be reconstructed from the public fighter trace.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationStatus {
    pub layer_animation_stopped: [bool; 5],
    pub tilt_frame: Option<f32>,
    pub tilt_rewound: bool,
    pub material_fade_complete: [bool; 6],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MatchMode {
    #[default]
    Versus,
    /// Stage_80225194 == 0xB0 or 0xFB; gm_8016ECE8 supplies the ratio.
    Boss { remaining_health_ratio: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalDestination {
    pub ground: Ground,
    pub mode: MatchMode,
    pub actions: Vec<StageAction>,
}
