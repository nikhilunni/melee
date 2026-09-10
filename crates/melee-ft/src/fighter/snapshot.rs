use super::{Fighter, FighterCore};
use melee_types::snapshot::{Snapshot, SnapshotSink};

/// Exactly harness/schema/fighter.yaml's 24 scalar keys. Player prefixes are
/// supplied by the caller's PrefixSink; RNG belongs to the scene.
impl Snapshot for FighterCore {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("kind", &i32::from(self.kind));
        sink.field("player_id", &self.player.id);
        sink.field("motion_id", &i32::from(self.motion_state.action));
        sink.field("facing_dir", &self.physics.facing);
        sink.field("self_vel", &self.physics.self_velocity);
        sink.field("kb_vel", &self.physics.knockback_velocity);
        sink.field("cur_pos", &self.physics.position);
        sink.field("ground_or_air", &i32::from(self.physics.ground_or_air));
        sink.field("cur_anim_frame", &self.animation.frame);
        sink.field("percent", &self.physics.percent);
        sink.field("jumps_used", &self.physics.jumps_used);
        sink.field("cpu.buttons", &self.cpu.buttons);
        sink.field("cpu.lstick_x", &self.cpu.stick[0]);
        sink.field("cpu.lstick_y", &self.cpu.stick[1]);
        sink.field("cpu.type", &self.cpu.mode);
        sink.field("cpu.level", &self.cpu.level);
        sink.field("cpu.behavior", &self.cpu.behavior);
        sink.field("cpu.timer", &self.cpu.reaction_timer);
    }
}

impl Snapshot for Fighter {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        self.core.snapshot(sink);
    }
}
