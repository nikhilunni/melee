//! Entry graphics read their positional joint before the caller attaches it.
use super::*;
use crate::scenario::Scenario;
use std::path::PathBuf;

#[test]
fn immediate_entry_graphics_keep_the_joint_read_before_attachment() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario = Scenario::load(
        &root.join("harness/scenarios/corpus_v3_fd_kirby_captainfalcon4_e1cda1301_p1.toml"),
    )
    .unwrap();
    let state = InitialState::from_savestate_traces(&scenario).unwrap();
    for immediate in [false, true] {
        let mut fighter = state.fighters[1].clone();
        let f = &mut **fighter.0;
        f.effect_state.dispatch_at_once = immediate;
        f.change_motion_state_keeping_effects(
            melee_ft::fighter::state::ActionId(355),
            &state.assets.fighters[1],
            0.0,
        )
        .unwrap();
        let command = f.commands.graphics.iter().find(|g| g.id == 0x41D).unwrap();
        assert_eq!(command.issued_joint.is_some(), immediate);
        if let Some(issued) = command.issued_joint {
            assert_eq!(issued.bone, 61);
            let before = issued.matrix;
            let root = f.animation.root;
            f.skeleton.set_translate_x(root, 100.0);
            assert_eq!(
                f.commands
                    .graphics
                    .iter()
                    .find(|g| g.id == 0x41D)
                    .unwrap()
                    .issued_joint
                    .unwrap()
                    .matrix,
                before
            );
        } else {
            // A hidden owner does not capture a queued command when its next
            // animation step would otherwise demand the outgoing joint.
            f.effect_state.dispatch_at_once = true;
            f.effect_state.invisible = true;
            f.step_animation(&state.assets.fighters[1]);
            assert!(f
                .commands
                .graphics
                .iter()
                .find(|g| g.id == 0x41D)
                .unwrap()
                .issued_joint
                .is_none());
        }
    }
}

