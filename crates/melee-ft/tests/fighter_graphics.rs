//! Command effect flags and common-bone selection are independent of particle simulation.
use melee_ef::request::EffectRequest;
mod fighter_support;
use fighter_support::Fixture;
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::Fighter;
use melee_types::combat::GraphicsCommand;
use melee_types::CommonMotionState;

#[test]
fn invisible_commands_do_not_advance_rng_or_the_rotating_bone_cursor() {
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let (tree, root) = fixture.model(0);
    let mut fighter = Fighter::prepare(
        Fixture::player(0),
        fixture.character(),
        &fixture.assets,
        tree,
        root,
        &fixture.map,
    );
    let command = GraphicsCommand {
        id: 9,
        bone: 0x8D,
        common_bone: true,
        item_bone: false,
        destroy_on_state_change: true,
        parameter: 0.0,
        offset: Vec3::ZERO,
        range: Vec3::ZERO,
    };
    let mut rng = HsdRng::new(1);
    fighter.effect_state.invisible = true;
    fighter.commands.graphics.push(command.clone());
    assert_eq!(
        fighter.resolve_graphics_commands(&fixture.assets, &mut rng),
        0
    );
    assert_eq!(rng.seed, 1);
    assert_eq!(fighter.effect_state.rotating_bone_index, 0);
    assert!(!fighter.effect_state.destroy_on_state_change);
    assert!(fighter.effects.is_empty());
    fighter.effect_state.invisible = false;
    let expected_bones = fixture
        .assets
        .rotating_effect_bones
        .into_iter()
        .chain([fixture.assets.rotating_effect_bones[0]]);
    for expected_bone in expected_bones {
        fighter.commands.graphics.push(command.clone());
        assert_eq!(
            fighter.resolve_graphics_commands(&fixture.assets, &mut rng),
            3,
            "zero ranges still draw X, Y, Z"
        );
        let EffectRequest::Graphics { bone, offset, .. } = fighter.effects.pop().unwrap() else {
            panic!("missing positional effect")
        };
        assert_eq!(bone, expected_bone);
        assert_eq!(offset, Vec3::ZERO);
    }
    assert_eq!(fighter.effect_state.rotating_bone_index, 1);
    assert!(fighter.effect_state.destroy_on_state_change);
    fighter
        .change_motion_state(CommonMotionState::Wait, &fixture.assets)
        .unwrap();
    assert!(!fighter.effect_state.destroy_on_state_change);
    assert_eq!(
        fighter.effect_state.rotating_bone_index, 1,
        "motion changes preserve the common-bone cycle"
    );
    assert_eq!(fighter.effects, [EffectRequest::DestroyOwned]);
}
