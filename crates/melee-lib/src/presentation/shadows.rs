//! Live collision segments used by the application's planar shadow pass.
use super::*;
pub(super) fn floors(game: &Match) -> [[f32; 4]; 2] {
    use melee_types::mp::{line_flag, line_kind};
    let state = game.engine.state();
    std::array::from_fn(|slot| {
        let fighter = &state.fighters[slot].0;
        let p = fighter.physics.position;
        let mut selected = [1.0, 0.0, -1.0, 0.0];
        let mut highest = f32::NEG_INFINITY;
        for (line, runtime) in state.map.data().lines.iter().zip(state.map.coll_lines()) {
            if runtime.flags & (line_kind::FLOOR | line_flag::ENABLED)
                != (line_kind::FLOOR | line_flag::ENABLED)
                || runtime.flags & (line_flag::EMPTY | line_flag::HIDDEN) != 0
            {
                continue;
            }
            let a = state.map.vertices()[usize::from(line.v0_idx)].pos;
            let b = state.map.vertices()[usize::from(line.v1_idx)].pos;
            let (left, right) = if a.x <= b.x { (a, b) } else { (b, a) };
            if p.x < left.x || p.x > right.x || right.x == left.x {
                continue;
            }
            // Presentation-only plane selection; no collision or simulation mutation.
            let y = left.y + (right.y - left.y) * (p.x - left.x) / (right.x - left.x);
            if y <= p.y + 1.0 && y > highest {
                highest = y;
                selected = [left.x, left.y, right.x, right.y];
            }
        }
        selected
    })
}
