use super::*;
use barrel::{Hold, Spin};

pub(super) fn parameters() -> Parameters {
    // GrKg.dat yakumono_param.
    Parameters {
        event_wait: [1200.0, 2400.0],
        klap_weight: 1.0,
        box_weight: 1.0,
        object_weight: 2.0,
        path_speed: 2.3,
        log_share: 0.75,
        repeat_chance: 0.3,
        hold_frames: [479.0, 480.0],
        catch_radius: 15.0,
        rest_frames: [180.0, 360.0],
        spin_acceleration: 4.0,
        spin_speed: 4.0,
        spin_frames: [90.0, 110.0],
        direction_weights: [1, 1, 10, 50, 10, 1, 1, 1],
        dim_frames: [300, 600],
        rate_step: 0.0025,
        rate_top: 0.5,
        bright_frames: [3000, 6000],
        launch_hit: [0, 361, 0, 0, 180, 0],
        catch_material: 0x41C20,
        log_landings: 60.0,
        log_frames: 2000.0,
        plank_limit: 5.0,
        plank_restore: 0.003,
        plank_speed_limit: 1.0,
        plank_slack: 1.0,
        plank_pull: 0.2,
        plank_kick: 1.0,
        end_tilt: 6.0,
        end_angle_limit: 10.0,
        end_jolt: 1.0,
        swing_damping: 0.04,
        swing_limit: 20.0,
    }
}

#[test]
fn a_kicked_plank_sags_and_springs_back() {
    let p = parameters();
    let mut bridge = Bridge::default();
    bridge.reset_angles();
    let left = -60.0;
    let right = 60.0;
    // x = 0 is the middle plank.
    bridge.land(&p, 0.0, left, right);
    assert!(bridge.planks[7].velocity > 0.0);
    let limit = DEG_TO_RAD * p.plank_limit;
    for _ in 0..60 {
        bridge.step(&p, [0.0, 0.0]);
        // Every angle stays within the limit, rest angles included.
        assert!(bridge.planks.iter().all(|plank| plank.angle.abs() <= limit));
    }
    assert!(bridge.planks[7].sag != 0.0);
    // The neighbours were pulled along.
    assert!(bridge.planks[6].angle != bridge.planks[6].rest);
    assert_eq!(bridge.line_offsets().count(), 17);
}

#[test]
fn a_swing_comes_to_rest() {
    let p = parameters();
    let mut swing = Swing {
        angle: 0.0,
        velocity: 0.05,
    };
    for _ in 0..2000 {
        swing.step(&p);
    }
    assert_eq!(swing, Swing::default());
}

#[test]
fn the_barrel_rests_spins_and_settles_on_a_weighted_facing() {
    let p = parameters();
    let mut rng = HsdRng::new(3);
    let mut barrel = Barrel::new(&p, &mut rng);
    assert_eq!((barrel.spin, barrel.hold, barrel.rate_phase), (Spin::Rest, Hold::Empty, 2));
    assert!((180..360).contains(&barrel.spin_timer));
    let mut phases = std::collections::BTreeSet::new();
    for _ in 0..2000 {
        barrel.step(&p, &mut rng);
        phases.insert(barrel.spin as u8);
    }
    assert_eq!(phases.len(), 4, "every spin phase ran");
    assert!(barrel.angle.abs() <= 6.2831855);
}

#[test]
fn the_launch_angle_turns_the_facing_a_quarter() {
    let p = parameters();
    let mut barrel = Barrel::new(&p, &mut HsdRng::new(0));
    barrel.angle = 0.0;
    assert_eq!(barrel.launch_degrees(), 57.295_78 * 1.570_796_4);
}
