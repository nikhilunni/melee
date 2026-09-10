//! Cold spawn expectations derived from the retail reset stores, separately
//! from importing the idle savestate. No oracle values are rewritten.
mod fighter_support;
use fighter_support::*;
use gekko_math::rng::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::{CpuState, Fighter, Interaction, SpawnContext, SpawnCounter};
use melee_types::snapshot::{SnapValue, Snapshot};
use std::collections::BTreeMap;

#[test]
fn fox_spawn_native() {
    let Some(mut fixture) = Fixture::load() else {
        return;
    };
    for player in 0..2 {
        let mut slot = Fixture::player(player);
        slot.position = fixture.spawn_points[usize::from(player)];
        assert_eq!(slot.position.y, 10.0);
        let (tree, root) = fixture.model(slot.costume);
        let mut rng = HsdRng::new(0x12345678);
        let mut counter = SpawnCounter(1);
        let f = Fighter::spawn(
            slot,
            fixture.character(),
            &fixture.assets,
            tree,
            root,
            SpawnContext {
                map: &mut fixture.map,
                rng: &mut rng,
                counter: &mut counter,
            },
        )
        .unwrap();
        // HSD_Randf (80380528) LCG: b3e97b5b; ftCo_800A101C's fmul/fctiwz
        // at 800A124C/800A1258 truncates 10*(b3e9/65536) to 7.
        assert_eq!(
            // Draws at 0x800A123C and 0x800B9718. The old one-draw
            // expectation was self-authored, not oracle-derived.
            rng.seed,
            0x5B3F58B2,
            "two CPU initialization draws, even for Human"
        );
        assert_eq!(counter.0, 2);
        assert_eq!(f.spawn_number, 1);
        let mut values = Vec::new();
        f.snapshot(&mut values);
        let actual: BTreeMap<_, _> = values.into_iter().collect();
        assert_eq!(actual.len(), 24);
        let mut expected = BTreeMap::new();
        for (key, value) in [
            ("kind", 1),
            ("motion_id", 29),
            ("ground_or_air", 1),
            ("cpu.lstick_x", 0),
            ("cpu.lstick_y", 0),
            ("cpu.type", 4),
            ("cpu.level", 1),
            ("cpu.behavior", 1),
            ("cpu.timer", 7),
        ] {
            expected.insert(key.to_owned(), SnapValue::I64(value));
        }
        for (key, value) in [
            ("player_id", u64::from(player)),
            ("jumps_used", 1),
            ("cpu.buttons", 0),
        ] {
            expected.insert(key.to_owned(), SnapValue::U64(value));
        }
        for key in [
            "self_vel.x",
            "self_vel.y",
            "self_vel.z",
            "kb_vel.x",
            "kb_vel.y",
            "kb_vel.z",
            "cur_pos.z",
            "cur_anim_frame",
            "percent",
        ] {
            expected.insert(key.to_owned(), SnapValue::F32(0));
        }
        expected.insert(
            "cur_pos.x".into(),
            SnapValue::f32(if player == 0 { -60.0 } else { 60.0 }),
        );
        // FD marker is y=10. The failed probe leaves Fighter.cur_pos unchanged.
        expected.insert("cur_pos.y".into(), SnapValue::f32(10.0));
        expected.insert(
            "facing_dir".into(),
            SnapValue::f32(if player == 0 { 1.0 } else { -1.0 }),
        );
        assert_eq!(actual, expected, "cold player {player}");
        assert_eq!(
            f.physics.previous_position,
            Vec3::new(if player == 0 { -60.0 } else { 60.0 }, 10.0, 0.0)
        );
        assert_eq!(f.status.interaction, Interaction::Idle);
        assert!(!f.status.disabled);
        assert_eq!(f.status.time_since_hit, -1);
        assert_eq!(f.status.time_since_smash.to_bits(), (-1.0f32).to_bits());
        assert_eq!(f.status.sword_trail, -1);
        assert_eq!(f.status.ledge_cooldown, 0);
        assert!(f.status.input_frozen);
        assert_eq!(f.thrown_hitbox.state, 2);
        assert_eq!(
            f.status.shield_health.to_bits(),
            fixture.assets.shield_health.to_bits()
        );
        assert!(f.capabilities.can_walljump);
        assert_eq!(f.character.registered_items.len(), 3);
        assert!(!f.character.blaster_present);
        assert_eq!(f.dynamics_first_bone, [0]);
        assert_eq!(f.bones.ecb.joints, [41, 55, 25, 13, 7, 4]);
        let expected = f.row(melee_types::CommonMotionState::Fall.into());
        assert_eq!(f.motion_state.action, expected.action);
        assert_eq!(f.motion_state.id, expected.id);
        assert_eq!(f.motion_row.animation, expected.animation);
        assert!(std::ptr::fn_addr_eq(f.motion_row.anim, expected.anim));
        assert!(std::ptr::fn_addr_eq(f.motion_row.iasa, expected.iasa));
        assert!(std::ptr::fn_addr_eq(f.motion_row.physics, expected.physics));
        assert!(std::ptr::fn_addr_eq(
            f.motion_row.collision,
            expected.collision
        ));
        assert!(std::ptr::fn_addr_eq(f.motion_row.camera, expected.camera));

        assert_eq!(f.ground_pose.0, 0, "Fall does not install ground IK");
        assert_eq!(f.collision.lock_frames, 10);
        assert_eq!(f.collision.data.floor.index, -1);
        let scale = f.skeleton.scale(root);
        assert_eq!(
            [scale.x.to_bits(), scale.y.to_bits(), scale.z.to_bits()],
            [0x3F75C28F; 3]
        );
        let bone = f.skeleton.bone(root, 67).unwrap();
        assert_eq!(
            f.skeleton.scale(bone).x.to_bits(),
            0x3F855556,
            "ftCommon_8007F6A4 fdivs"
        );
    }
}

#[test]
fn cpu_init_is_distinct_from_player_control() {
    for (mode, behavior) in [(1, 12), (25, 12), (3, 1), (15, 0), (4, 1)] {
        let mut rng = HsdRng::new(0x12345678);
        let cpu = CpuState::initialize(mode, 1, &mut rng);
        assert_eq!(cpu.behavior, behavior);
        assert_eq!(cpu.reaction_timer, 7);
        // Draws at 0x800A123C and 0x800B9718. The old one-draw
        // expectation was self-authored, not oracle-derived.
        assert_eq!(rng.seed, 0x5B3F58B2);
    }
}
#[test]
fn spawn_counter_skips_zero_after_wrapping() {
    let mut counter = SpawnCounter(u32::MAX);
    assert_eq!(counter.allocate(), u32::MAX);
    assert_eq!(counter.allocate(), 1);
    assert_eq!(counter.0, 2);
}

#[test]
fn unsupported_interactions_and_installed_callbacks_fail_loudly() {
    use melee_ft::fighter::state::unimplemented_anim;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let mut fighter = fixture.prepared(Fixture::player(0));
    for interaction in [
        Interaction::Hitlag,
        Interaction::HeldItem,
        Interaction::Grab,
        Interaction::Damage,
        Interaction::Death,
        Interaction::StatusEffect,
        Interaction::Accessory,
        Interaction::Attack,
        Interaction::AsyncEffect,
        Interaction::StageHazard,
        Interaction::FighterOverlap,
        Interaction::CoinMatch,
    ] {
        fighter.status.interaction = interaction;
        assert!(catch_unwind(AssertUnwindSafe(|| fighter.proc_status())).is_err());
    }
    // Shield is now supported: actual input installs its state and the proc arm runs.
    fighter.status.interaction = Interaction::Idle;
    fighter
        .change_motion_state(melee_types::CommonMotionState::Wait, &fixture.assets)
        .unwrap();
    fighter.proc_input(
        &fixture.assets,
        &melee_ft::input::PadSample {
            buttons: melee_ft::input::Buttons::L,
            left_trigger: 1.0,
            ..Default::default()
        },
    );
    assert!(fighter.shield.enabled);
    assert!(matches!(
        fighter.state_data,
        melee_ft::fighter::MotionData::Guard(_)
    ));
    fighter.status.interaction = Interaction::Shield;
    fighter.proc_status();
    fighter.proc_process_hit(&fixture.assets);
    fighter.status.interaction = Interaction::Idle;
    // Dispatch must consult the installed callback, not just motion_id=Wait.
    fighter.motion_row.anim = unimplemented_anim;
    assert!(catch_unwind(AssertUnwindSafe(
        || fighter.proc_anim(&fixture.assets, &mut HsdRng::new(1))
    ))
    .is_err());
}
