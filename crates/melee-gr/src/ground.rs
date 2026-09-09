//! Named state corresponding to FD's two Ground union views.
use crate::last::{
    background::BackgroundMotion,
    lights::{dark_palette, ColorFade, Rgb},
};

/// The 17 cases of `grLast_8021B920`, retail 0x8021B920.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Phase {
    LayeredStart = 1,
    LayeredTransition1,
    LayeredContinuation,
    LayeredTransition2,
    LayeredHold2,
    LayeredTransition3,
    LayeredHold3,
    LayeredTransition4,
    Tilt,
    Flash,
    Quake,
    QuakeHold,
    FadeOutLayers,
    FadeInStars,
    Stars,
    FadeOutStars,
    FadeInLayers,
}
impl TryFrom<u16> for Phase {
    type Error = &'static str;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        use Phase::*;
        [
            LayeredStart,
            LayeredTransition1,
            LayeredContinuation,
            LayeredTransition2,
            LayeredHold2,
            LayeredTransition3,
            LayeredHold3,
            LayeredTransition4,
            Tilt,
            Flash,
            Quake,
            QuakeHold,
            FadeOutLayers,
            FadeInStars,
            Stars,
            FadeOutStars,
            FadeInLayers,
        ]
        .get(value.wrapping_sub(1) as usize)
        .copied()
        .ok_or("FD phase must be 1..17")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ground {
    pub waiting_for_start: bool,
    pub demo_frozen: bool,
    pub phase: Phase,
    pub elapsed: f32,
    pub transition_enabled: bool,
    pub live_maps: [bool; 10],
    pub fade: ColorFade,
    pub fog: Rgb,
    pub environment_colors: [Rgb; 9],
    pub background: Option<BackgroundMotion>,
    pub fog_enabled: bool,
}
impl Ground {
    /// `grLast_8021A9C4`, retail 0x8021A9C4; phase entry is performed by init.rs.
    pub fn controller(fog: Rgb) -> Self {
        Self {
            waiting_for_start: true,
            demo_frozen: false,
            phase: Phase::LayeredStart,
            elapsed: 0.0,
            transition_enabled: false,
            live_maps: [false; 10],
            fade: ColorFade::default(),
            fog,
            environment_colors: dark_palette(),
            background: None,
            fog_enabled: true,
        }
    }
    /// `grLast_8021A9AC`, retail 0x8021A9AC, deferred by Ground_801C10B8
    /// and invoked from Ground_801C0FB8 at match start (after fighter creation).
    pub fn start(&mut self) {
        self.waiting_for_start = false;
    }
    /// `grLast_OnDemoInit`, retail 0x8021A620, for demo kind 26 only.
    pub fn demo_init(&mut self, kind: i32) {
        const FD_DEMO: i32 = 26;
        if kind == FD_DEMO {
            self.live_maps[4..].fill(false);
            self.background = None;
            self.demo_frozen = true;
        }
    }
}

use melee_types::snapshot::{Snapshot, SnapshotSink};
impl Snapshot for Ground {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("stage.map[3].waiting_for_start", &self.waiting_for_start);
        sink.field("stage.map[3].demo_frozen", &self.demo_frozen);
        sink.field("stage.map[3].phase", &(self.phase as u16));
        sink.field("stage.map[3].elapsed", &self.elapsed);
        sink.field("stage.map[3].transition_enabled", &self.transition_enabled);
        sink.field("stage.map[3].fade_complete", &self.fade.complete);
        // C leaves these floats uncleared until begin(). Do not compare undefined
        // heap bytes after cold initialization; a capture may retain them separately.
        if !self.fade.complete {
            for i in 0..3 {
                sink.field(
                    &format!("stage.map[3].fade_target[{i}]"),
                    &self.fade.target[i],
                );
                sink.field(
                    &format!("stage.map[3].fade_current[{i}]"),
                    &self.fade.current[i],
                );
            }
            sink.field("stage.map[3].fade_remaining", &self.fade.remaining);
        }
        if let Some(bg) = &self.background {
            for (name, value) in [
                ("pitch", bg.pitch),
                ("yaw", bg.yaw),
                ("pitch_speed", bg.pitch_speed),
                ("yaw_speed", bg.yaw_speed),
                ("pitch_acceleration", bg.pitch_acceleration),
                ("yaw_acceleration", bg.yaw_acceleration),
                ("amplitude", bg.amplitude),
            ] {
                sink.field(&format!("stage.map[7].{name}"), &value);
            }
            sink.field("stage.map[7].generator_present", &bg.generator_present);
        }
        for (i, &live) in self.live_maps.iter().enumerate() {
            sink.field(&format!("stage.live_maps[{i}]"), &live);
        }
        for (i, &channel) in self.fog.iter().enumerate() {
            sink.field(&format!("stage.fog[{i}]"), &channel);
        }
    }
}
