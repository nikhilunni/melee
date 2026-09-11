//! Complete the Versus HUD setup before the first scheduler tick.
use anyhow::Result;
use hsd_anim::load::{attach_anim_joint, load_joint_tree};
use hsd_archive::{desc::model, Archive};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_if::StockDisplay;
#[cfg(test)]
use std::path::Path;

/// gm_Scene_Vs_OnEnter (8016E934): after music and countdown creation,
/// ifStatus_802F665C builds the HUD before HSD_GObj_80390CFC's first tick.
/// Both cold and saved match-start setup complete this same pending work.
pub(super) fn create(archive: &Archive, stocks: [u8; 2]) -> Result<[Option<StockDisplay>; 2]> {
    let layout = model::read_scene_joint(archive, "ScInfDmg_scene_data")?;
    let (mut layout_tree, layout_root) = load_joint_tree(archive, &layout)?;
    let (joint, animation) = model::read_dynamic_model_animation(archive, "Stc_scemdls", 0, 0)?;
    let mut displays = [None, None];
    for (player, display) in displays.iter_mut().enumerate() {
        // gm_SetupRulesDefaults selects four HUD slots even with two fighters;
        // ifAll_802F343C(4) uses descendants 2..5, indexed by player slot.
        // lb_8000B1CC uses the audited HSD world matrix, including ancestors.
        let anchor = layout_tree.bone(layout_root, 2 + player).unwrap();
        let matrix = layout_tree.get_mtx(anchor);
        let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        let (mut tree, root) = load_joint_tree(archive, &joint)?;
        attach_anim_joint(&mut tree, root, &animation, archive)?;
        tree.req_anim_all(root, 0.0);
        tree.set_translate(root, &position);
        // ifStock_802F98E8 initializes absent icons at the completed frame;
        // its inline fn_802F9410 then evaluates each requested icon pose.
        let frames = std::array::from_fn(|i| {
            if i < usize::from(stocks[player]) {
                0
            } else {
                10
            }
        });
        for (i, &frame) in frames.iter().enumerate() {
            let icon = tree.bone(root, i + 1).unwrap();
            tree.req_anim_all(icon, f32::from(frame));
        }
        tree.anim_all::<RetailTrig>(root);
        let root_position = tree.get(root).translate;
        let icon_positions = std::array::from_fn(|i| {
            let local = tree.get(tree.bone(root, i + 1).unwrap()).translate;
            // ifStock_802F8298: separate fadds, no fused instructions.
            Vec3::new(
                local.x + root_position.x,
                local.y + root_position.y,
                local.z + root_position.z,
            )
        });
        *display = Some(StockDisplay {
            icon_positions,
            animation_frames: frames,
            animate_losses: true,
        });
    }
    Ok(displays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initial_state::InitialState, scenario::Scenario, scene_fighter::with_fighter};

    #[test]
    fn created_stock_icons_match_the_independent_post_hud_savestate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let scenario =
            Scenario::load(&root.join("harness/scenarios/match_fd_marth_scripted.toml")).unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        let saved = InitialState::from_savestate_traces(&scenario).unwrap();
        let stocks =
            std::array::from_fn(|slot| with_fighter!(&saved.fighters[slot], |f| f.player.stocks));
        let created = create(&saved.assets.interface, stocks).unwrap();
        for (slot, (actual, expected)) in created.iter().zip(&saved.stock_displays).enumerate() {
            let actual = actual.as_ref().unwrap();
            let expected = expected.as_ref().unwrap();
            for (icon, (actual, expected)) in actual
                .icon_positions
                .iter()
                .zip(&expected.icon_positions)
                .enumerate()
            {
                let bits = |v: &Vec3| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
                assert_eq!(bits(actual), bits(expected), "player {slot} icon {icon}");
            }
            assert_eq!(
                actual.animation_frames, expected.animation_frames,
                "player {slot}"
            );
            assert_eq!(
                actual.animate_losses, expected.animate_losses,
                "player {slot}"
            );
        }
    }
}
