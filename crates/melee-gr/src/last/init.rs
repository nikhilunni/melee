use super::{
    background::BackgroundMotion,
    lights::{bright_palette, dark_palette},
    FinalDestination, MatchMode, StageAction,
};
use crate::{
    desc::StageDesc,
    ground::{Ground, Phase},
};
use gekko_math::HsdRng;

const LAYERS: std::ops::Range<u8> = 4..9;
const BASE_ANIMATIONS: [u8; 5] = [0, 4, 6, 8, 10];

impl FinalDestination {
    /// `grLast_OnInit`, retail 0x8021A740. Cold init consumes four direct draws;
    /// Ground's match-load draw and fighter-init draws belong to the caller.
    pub fn initialize(desc: &StageDesc, rng: &mut HsdRng) -> Self {
        let mut stage = Self {
            ground: Ground::controller(desc.initial_fog),
            mode: MatchMode::Versus,
            actions: Vec::new(),
        };
        for map in 0..4 {
            stage.create_map(map, rng);
        }
        stage.enter_phase(Phase::LayeredStart, rng);
        stage
    }
    /// `grLast_8021A7F4`, retail 0x8021A7F4. Ground_801C1CD0/1D38
    /// registrations precede init, which precedes the separate stage proc.
    pub(crate) fn create_map(&mut self, map: u8, rng: &mut HsdRng) {
        self.ground.live_maps[map as usize] = true;
        self.actions.push(StageAction::CreateMap(map));
        match map {
            0..=3 => self.play(map, 0, 0, true),
            // grLast_8021AC30, 0x8021AC30: other model initializers only
            // initialize material state and select render priority 2.
            7 => self.ground.background = Some(BackgroundMotion::initialize(rng)),
            _ => {}
        }
    }
    fn destroy_map(&mut self, map: u8) {
        self.ground.live_maps[map as usize] = false;
        self.actions.push(StageAction::DestroyMap(map));
        if map == 7 {
            self.ground.background = None;
        }
    }
    pub(crate) fn play(&mut self, map: u8, joint: u16, animation: u8, subtree: bool) {
        self.actions.push(StageAction::PlayAnimation {
            map,
            joint,
            animation,
            subtree,
        });
    }
    /// do_anime inline in `grLast_8021B920`, retail 0x8021BB2C et seq.
    fn layer_animation(&mut self, map: u8, animation: u8) {
        self.play(map, 1, animation, false);
        self.actions
            .push(StageAction::StopAnimation { map, joint: 1 });
    }
    fn create_layers(&mut self, rng: &mut HsdRng) {
        for map in LAYERS {
            if !self.ground.live_maps[map as usize] {
                self.create_map(map, rng);
                self.play(map, 2, BASE_ANIMATIONS[(map - 4) as usize], true);
                self.layer_animation(map, 0);
            }
        }
    }
    fn animate_layers(&mut self, animation: u8) {
        for map in LAYERS {
            self.layer_animation(map, animation);
        }
    }
    fn fade(&mut self, color: [u8; 3], frames: f32) {
        self.ground.fade.begin(self.ground.fog, color, frames);
    }
    /// `grLast_8021B920`, retail 0x8021B920. All 17 branches; engine
    /// animation/material/generator calls remain ordered StageActions.
    pub fn enter_phase(&mut self, phase: Phase, rng: &mut HsdRng) {
        use Phase::*;
        self.ground.elapsed = 0.0;
        self.ground.phase = phase;
        match phase {
            LayeredStart => {
                self.create_layers(rng);
                self.play(4, 16, 0, false);
                self.ground.transition_enabled = false;
                self.ground.environment_colors = dark_palette();
            }
            LayeredTransition1 => {
                self.animate_layers(1);
                self.actions
                    .push(StageAction::RequestAnimation { map: 4, joint: 16 });
                self.actions
                    .push(StageAction::RemoveJointGenerators { map: 4, joint: 0 });
                self.ground.transition_enabled = true;
            }
            LayeredContinuation | LayeredTransition2 | LayeredHold2 | LayeredTransition3
            | LayeredHold3 | LayeredTransition4 => {
                self.animate_layers(phase as u8 - 1);
                self.ground.transition_enabled = !matches!(phase, LayeredHold2 | LayeredHold3);
            }
            Tilt => {
                self.animate_layers(8);
                self.actions.push(StageAction::CreateTiltGenerator);
                // Actual allocation success must be supplied by the engine.
                self.ground.transition_enabled = false;
            }
            Flash => {
                self.fade([170, 202, 255], 120.0);
                self.actions
                    .push(StageAction::RequestAnimation { map: 7, joint: 5 });
                self.actions.push(StageAction::RemoveTiltGenerator);
                if let Some(bg) = &mut self.ground.background {
                    bg.generator_present = false;
                }
                self.ground.transition_enabled = true;
                self.ground.environment_colors = bright_palette();
            }
            Quake | QuakeHold => {
                self.animate_layers(if phase == Quake { 9 } else { 10 });
                self.ground.transition_enabled = phase == Quake;
            }
            FadeOutLayers => {
                self.fade([200; 3], 120.0);
                for map in LAYERS {
                    self.actions
                        .push(StageAction::MaterialFade { map, script: 0 });
                }
                self.ground.transition_enabled = true;
            }
            FadeInStars => {
                self.fade([0; 3], 60.0);
                for map in LAYERS {
                    self.destroy_map(map);
                }
                self.create_map(9, rng);
                self.play(9, 0, 0, true);
                self.actions
                    .push(StageAction::MaterialFade { map: 9, script: 1 });
                self.ground.fog_enabled = false;
                self.ground.transition_enabled = true;
                self.ground.environment_colors = dark_palette();
            }
            Stars => self.ground.transition_enabled = false,
            FadeOutStars => {
                self.fade([0; 3], 60.0);
                self.actions
                    .push(StageAction::MaterialFade { map: 9, script: 2 });
                self.ground.transition_enabled = true;
            }
            FadeInLayers => {
                for map in LAYERS {
                    self.create_map(map, rng);
                    self.play(map, 2, BASE_ANIMATIONS[(map - 4) as usize], true);
                    self.layer_animation(map, 0);
                    self.actions
                        .push(StageAction::MaterialFade { map, script: 3 });
                }
                self.destroy_map(9);
                self.ground.fog_enabled = true;
                self.ground.transition_enabled = true;
            }
        }
    }
}
