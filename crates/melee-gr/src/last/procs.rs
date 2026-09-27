//! Scheduler registrations, in HSD's (s_link, p_link, GObj order, proc order).
use super::{lights::choose_flicker, AnimationStatus, FinalDestination, MatchMode, StageAction};
use crate::ground::Phase;
use gekko_math::HsdRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcRegistration {
    pub s_link: u8,
    pub p_link: u8,
    pub p_priority: u8,
    pub map_id: Option<u8>,
    pub callback: &'static str,
    pub address: u32,
}
const CALLBACKS: [(&str, u32); 10] = [
    ("grLast_8021A914", 0x8021A914),
    ("grLast_8021A968", 0x8021A968),
    ("grLast_8021A9A4", 0x8021A9A4),
    ("grLast_8021AAB0", 0x8021AAB0),
    ("grLast_8021AB80", 0x8021AB80),
    ("grLast_8021ABD4", 0x8021ABD4),
    ("grLast_8021AC28", 0x8021AC28),
    ("grLast_8021ADD0", 0x8021ADD0),
    ("grLast_8021B28C", 0x8021B28C),
    ("grLast_8021B2E0", 0x8021B2E0),
];
pub fn map_callback(map: u8) -> u32 {
    CALLBACKS[usize::from(map)].1
}
fn registration(
    s: u8,
    p: u8,
    map: Option<u8>,
    callback: &'static str,
    address: u32,
) -> ProcRegistration {
    ProcRegistration {
        s_link: s,
        p_link: p,
        p_priority: 0,
        map_id: map,
        callback,
        address,
    }
}
impl FinalDestination {
    /// Ground_GetStageGObj (0x801C14D0), grLast_8021A7F4 (0x8021A7F4),
    /// Ground_801C466C (0x801C466C), grZakoGenerator_801CAE04 (0x801CAE04),
    /// Ground_801C0FB8 (0x801C0FB8). Re-query after stage creation/deletion.
    /// All maps have p_priority 0; map 3 is created before its children.
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        let mut result = vec![registration(0, 3, None, "Ground_801C461C", 0x801C461C)];
        if !self.ground.waiting_for_start {
            result.push(registration(0, 4, None, "fn_801CADBC", 0x801CADBC));
        }
        for (map, &live) in self.ground.live_maps.iter().enumerate() {
            if live {
                result.push(registration(
                    1,
                    5,
                    Some(map as u8),
                    "Ground_801C1CD0",
                    0x801C1CD0,
                ));
            }
        }
        for (map, &live) in self.ground.live_maps.iter().enumerate() {
            if live {
                result.push(registration(
                    4,
                    5,
                    Some(map as u8),
                    "Ground_801C1D38",
                    0x801C1D38,
                ));
                let (name, address) = CALLBACKS[map];
                result.push(registration(4, 5, Some(map as u8), name, address));
            }
        }
        if !self.ground.waiting_for_start {
            result.push(registration(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        }
        result
    }

    /// Ground_801C0FB8's GObjs, which a stage still waiting for its start
    /// (Versus countdown) creates only then: the zako generator's
    /// fn_801CADBC and the common Ground_801C0C2C (the Sudden Death rain).
    pub fn start_proc_table(&self) -> Vec<ProcRegistration> {
        if !self.ground.waiting_for_start {
            return Vec::new();
        }
        vec![
            registration(0, 4, None, "fn_801CADBC", 0x801CADBC),
            registration(10, 5, None, "Ground_801C0C2C", 0x801C0C2C),
        ]
    }

    /// All ten `StageCallbacks.gobj_proc` functions (addresses in proc_table).
    /// The eight empty C callbacks deliberately do nothing. Engine wrappers
    /// at s_link 0/1/10 are separate entries, not implicitly run here.
    pub fn run_stage_proc(&mut self, map: u8, status: &AnimationStatus, rng: &mut HsdRng) -> u32 {
        if !self
            .ground
            .live_maps
            .get(map as usize)
            .copied()
            .unwrap_or(false)
        {
            return 0;
        }
        match map {
            3 => self.tick_controller(status, rng),
            7 => {
                let bg = self
                    .ground
                    .background
                    .as_mut()
                    .expect("live map 7 has motion state");
                let draws = bg.tick(rng);
                if bg.generator_present {
                    self.actions.push(StageAction::UpdateTiltGeneratorTransform);
                }
                draws
            }
            _ => 0,
        }
    }
    /// `grLast_8021AAB0`, retail 0x8021AAB0. Map collision and dynamic
    /// callbacks following it do not change RNG for static FD geometry.
    fn tick_controller(&mut self, status: &AnimationStatus, rng: &mut HsdRng) -> u32 {
        if self.ground.demo_frozen || self.ground.waiting_for_start {
            return 0;
        }
        let mut draws = self.update_phase(rng);
        if self.ground.transition_enabled {
            if let Some(phase) = self.next_phase(status) {
                let before = self.ground.background.is_some();
                self.enter_phase(phase, rng);
                if !before && self.ground.background.is_some() {
                    draws += 4;
                }
            }
        }
        draws
    }
    /// `grLast_8021B2E8`, retail 0x8021B2E8. Test the old timer before +1.
    pub fn update_phase(&mut self, rng: &mut HsdRng) -> u32 {
        use Phase::*;
        match self.mode {
            MatchMode::Versus => {
                const HOLD_FRAMES: f32 = 1800.0;
                if self.ground.elapsed > HOLD_FRAMES {
                    self.ground.transition_enabled = true;
                }
            }
            MatchMode::Boss {
                remaining_health_ratio: ratio,
            } => {
                let threshold = match self.ground.phase {
                    LayeredStart => Some(0.7),
                    LayeredContinuation => Some(0.6),
                    LayeredHold2 => Some(0.5),
                    LayeredHold3 => Some(0.4),
                    Tilt => Some(0.3),
                    QuakeHold => Some(0.2),
                    _ => None,
                };
                if threshold.is_some_and(|threshold| ratio < threshold) {
                    self.ground.transition_enabled = true;
                }
            }
        }
        self.ground.elapsed += 1.0;
        let mut draws = 0;
        match self.ground.phase {
            Tilt => {
                if let Some(bg) = &mut self.ground.background {
                    // retail 0x8021B478: fadds; constant .sdata2 0x804DBBD0.
                    bg.amplitude = (bg.amplitude + 1.0 / 180.0).min(1.0);
                }
            }
            Flash => {
                if let Some(bg) = &mut self.ground.background {
                    // retail 0x8021B4A4: fsubs; 1/58 is rounded at compile time.
                    bg.amplitude = (bg.amplitude - 1.0 / 58.0).max(0.0);
                }
                self.actions.push(StageAction::Quake);
            }
            Quake => self.actions.push(StageAction::Quake),
            Stars if self.ground.fade.complete => {
                let (color, frames, n) = choose_flicker(self.ground.fog, rng);
                self.ground.fade.begin(self.ground.fog, color, frames);
                draws = n;
            }
            _ => {}
        }
        self.ground.fade.tick(&mut self.ground.fog);
        draws
    }
    /// `grLast_8021B5C4`, retail 0x8021B5C4. The caller gates this on
    /// transition_enabled; material completion is Ground.x10 bit 4.
    pub fn next_phase(&self, status: &AnimationStatus) -> Option<Phase> {
        use Phase::*;
        let stopped = status.layer_animation_stopped.iter().all(|&v| v);
        match self.ground.phase {
            LayeredStart => Some(LayeredTransition1),
            LayeredTransition1 if stopped => Some(LayeredContinuation),
            LayeredContinuation => Some(LayeredTransition2),
            LayeredTransition2 if stopped => Some(LayeredHold2),
            LayeredHold2 => Some(LayeredTransition3),
            LayeredTransition3 if stopped => Some(LayeredHold3),
            LayeredHold3 => Some(LayeredTransition4),
            LayeredTransition4 if stopped => Some(Tilt),
            Tilt if status.tilt_frame == Some(100.0) => Some(Flash),
            Flash if status.tilt_rewound => Some(Quake),
            Quake if stopped => Some(QuakeHold),
            QuakeHold => Some(FadeOutLayers),
            FadeOutLayers if self.ground.fade.complete && status.material_fade_complete[4] => {
                Some(FadeInStars)
            }
            FadeInStars if status.material_fade_complete[5] => Some(Stars),
            Stars => Some(FadeOutStars),
            FadeOutStars if self.ground.fade.complete && status.material_fade_complete[5] => {
                Some(FadeInLayers)
            }
            FadeInLayers if status.material_fade_complete[0] => Some(LayeredStart),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Ground;
    fn stage() -> FinalDestination {
        FinalDestination {
            ground: Ground::controller([0; 3]),
            mode: MatchMode::Versus,
            actions: Vec::new(),
        }
    }
    #[test]
    fn timer_uses_strict_comparison_before_increment() {
        let mut stage = stage();
        let mut rng = HsdRng::new(1);
        stage.ground.elapsed = 1800.0;
        stage.update_phase(&mut rng);
        assert!(!stage.ground.transition_enabled);
        stage.update_phase(&mut rng);
        assert!(stage.ground.transition_enabled);
        assert_eq!(stage.ground.elapsed, 1802.0);
        assert_eq!(rng.seed, 1);
    }
    #[test]
    fn animation_transition_requires_all_five_layers() {
        let mut stage = stage();
        stage.ground.phase = Phase::LayeredTransition1;
        let mut status = AnimationStatus {
            layer_animation_stopped: [true; 5],
            ..Default::default()
        };
        status.layer_animation_stopped[4] = false;
        assert_eq!(stage.next_phase(&status), None);
        status.layer_animation_stopped[4] = true;
        assert_eq!(stage.next_phase(&status), Some(Phase::LayeredContinuation));
    }
    #[test]
    fn new_background_runs_after_controller_in_same_slink() {
        let mut stage = stage();
        let mut rng = HsdRng::new(1);
        stage.ground.start();
        stage.ground.live_maps[..4].fill(true);
        stage.enter_phase(Phase::FadeInLayers, &mut rng);
        let rows = stage.proc_table();
        let maps: Vec<_> = rows
            .iter()
            .filter(|p| p.s_link == 1)
            .map(|p| p.map_id.unwrap())
            .collect();
        assert_eq!(maps, (0..9).collect::<Vec<_>>());
        let stage_callbacks: Vec<_> = rows
            .iter()
            .filter(|p| p.s_link == 4)
            .map(|p| (p.map_id.unwrap(), p.address))
            .collect();
        assert_eq!(&stage_callbacks[6..8], &[(3, 0x801C1D38), (3, 0x8021AAB0)]);
        assert_eq!(
            &stage_callbacks[14..16],
            &[(7, 0x801C1D38), (7, 0x8021ADD0)]
        );
    }
}
