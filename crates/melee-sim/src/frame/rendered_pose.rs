//! On-demand diagnostic poses; matrix setup is performed on a clone, never
//! on the live simulation or in its tick path.
use super::Simulation;

pub struct RenderedPose {
    /// Animation frame and world position, preserving signed zero.
    pub key: [u32; 4],
    /// Bone table order, twelve row-major matrix words per bone.
    pub matrices: Vec<[u32; 12]>,
}

impl Simulation {
    /// Evaluate the display-demanded pose of a fighter after a completed tick.
    pub fn rendered_fighter_pose(&self, player: usize) -> RenderedPose {
        let runtime = &self.runtime;
        crate::scene_fighter::with_fighter!(&runtime.state.fighters[player], |fighter| {
            let position = fighter.physics.position;
            let key =
                [fighter.animation.frame, position.x, position.y, position.z].map(f32::to_bits);
            let mut tree = fighter.skeleton.clone();
            let matrices = fighter
                .animation
                .parts
                .iter()
                .map(|part| {
                    tree.setup_matrix(part.joint);
                    std::array::from_fn(|i| tree.get(part.joint).mtx.0[i / 4][i % 4].to_bits())
                })
                .collect();
            RenderedPose { key, matrices }
        })
    }
}
