//! Raw fighter scratch assertions supplement M5's 49-key gate. Only the initial
//! state and pad samples drive simulation; subsequent retail bytes are assertions.
use super::*;
use crate::scenario::Scenario;
use melee_coll::hitbox::CapsulePhase;
use melee_ft::fighter::{Fighter, MotionData};
use std::{io::BufRead, path::Path};

const REFLECTOR_INPUT_SCENARIOS: [&str; 7] = [
    "reflectorturn_fd_fox",
    "reflectorturn_release_fd_fox",
    "airreflectorturn_fd_fox",
    "airreflectorturn_priority_fd_fox",
    "airreflectorturn_landing_fd_fox",
    "airreflectorjc_fd_fox",
    "airreflectortapjc_fd_fox",
];

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn vector(v: Vec3, bytes: &[u8], offset: usize) {
    for (axis, value) in [v.x, v.y, v.z].into_iter().enumerate() {
        assert_eq!(
            value.to_bits(),
            word(bytes, offset + axis * 4),
            "vector {offset:#x} axis {axis}"
        );
    }
}
fn compare(f: &Fighter, bytes: &[u8]) {
    for (actual, offset, label) in [
        (f.status.shield_health, 0x1998, "shield health"),
        (f.shield.lightshield, 0x199c, "lightshield amount"),
        (
            f.physics.ground_shield_knockback_velocity,
            0xf4,
            "attacker shield pushback",
        ),
        (f.physics.ground_velocity, 0xec, "ground velocity"),
        (f.animation.speed, 0x89c, "animation rate"),
    ] {
        assert_eq!(actual.to_bits(), word(bytes, offset), "{label}");
    }
    vector(f.physics.shield_knockback_velocity, bytes, 0x98);
    assert_eq!(
        f.combat.hitlag_remaining.to_bits(),
        word(bytes, 0x195c),
        "hitlag countdown"
    );
    if f.combat.clank.facing != 0.0 {
        assert_eq!(
            f.combat.clank.facing.to_bits(),
            word(bytes, 0x1920),
            "retained clank facing"
        );
        assert_eq!(
            f.combat.clank.damage as u32,
            word(bytes, 0x1918),
            "consumed clank damage"
        );
        assert_eq!(
            f.combat.clank.duration.to_bits(),
            word(bytes, 0x191c),
            "consumed clank duration"
        );
    }
    if let MotionData::Rebound(rebound) = &f.state_data {
        assert_eq!(
            rebound.recoil.to_bits(),
            word(bytes, 0x2340),
            "rebound recoil"
        );
        assert_eq!(
            rebound.animation_rate.to_bits(),
            word(bytes, 0x2344),
            "rebound rate"
        );
        assert_eq!(
            f.physics.secondary_ground_acceleration.to_bits(),
            word(bytes, 0xe8),
            "clank ground impulse"
        );
    }
    if let MotionData::Guard(guard) = &f.state_data {
        for (value, offset) in [
            (guard.dash_item_throw_frames, 0x2360),
            (guard.grab_delay, 0x2364),
        ] {
            assert_eq!(value as u32, word(bytes, offset), "guard delay {offset:x}");
        }
    }
    if let MotionData::Damage(damage) = &f.state_data {
        assert_eq!(
            damage.hitstun.to_bits(),
            word(bytes, 0x2340),
            "hitstun countdown"
        );
    }
    for (id, hit) in f.commands.throw_hitboxes.iter().enumerate() {
        if let Some(hit) = hit {
            let offset = 0xdf4 + id * 0x138;
            assert_eq!(
                hit.damage.to_bits(),
                word(bytes, offset + 12),
                "throw damage"
            );
            for (actual, field) in [
                (u32::from(hit.angle), 0x20),
                (u32::from(hit.growth), 0x24),
                (u32::from(hit.weight_knockback), 0x28),
                (u32::from(hit.base_knockback), 0x2c),
                (i32::from(hit.element) as u32, 0x30),
                // ftAction_80071E04 stores severity at +0x38 (0x80071ED0) and kind at
                // +0x3C (0x80071EE0); +0x34 is lb/types.h's `x34`. The earlier +0x34/+0x38
                // reading was a comparator mistake, never exercised before ThrowB.
                (u32::from(hit.sound_severity), 0x38),
                (u32::from(hit.sound_kind), 0x3c),
            ] {
                assert_eq!(
                    actual,
                    word(bytes, offset + field),
                    "throw field {field:#x}"
                );
            }
        }
    }
    for (id, hit) in f.commands.hitboxes.iter().enumerate() {
        let offset = 0x914 + id * 0x138;
        let Some(hit) = hit else {
            assert_eq!(word(bytes, offset), 0, "disabled hitbox {id}");
            continue;
        };
        let phase = match hit.phase {
            CapsulePhase::Enabled => 1,
            CapsulePhase::FirstPosition => 2,
            CapsulePhase::Sweeping => 3,
        };
        assert_eq!(phase, word(bytes, offset), "hitbox phase");
        assert_eq!(
            hit.descriptor.damage.to_bits(),
            word(bytes, offset + 12),
            "hitbox damage"
        );
        assert_eq!(
            hit.descriptor.radius.to_bits(),
            word(bytes, offset + 0x1c),
            "hitbox radius"
        );
        vector(hit.descriptor.offset, bytes, offset + 0x10);
        vector(hit.position, bytes, offset + 0x4c);
        vector(hit.previous_position, bytes, offset + 0x58);
    }
}
#[test]
fn jab_hitboxes_hitlag_and_hitstun_match_retail_scratch() {
    replay_scratch("jab_fd_marth");
}
#[test]
fn fox_jab_hitboxes_hitlag_and_hitstun_match_retail_scratch() {
    replay_scratch("jab_fd_fox");
}
#[test]
fn up_tilt_hitboxes_hitlag_and_hitstun_match_retail_scratch() {
    replay_scratch("utilt_fd_marth");
}
#[test]
fn shield_hitboxes_and_hitlag_match_retail_scratch() {
    replay_scratch("shieldhit_fd_marth");
}
#[test]
fn catch_startup_and_throw_hitbox_commands_match_retail_scratch() {
    replay_scratch_until("grab_fd_marth", 127);
}
#[test]
fn capture_back_throw_and_missed_tech_match_retail_scratch() {
    replay_scratch("grab_fd_marth");
}
#[test]
fn back_throw_and_tech_match_retail_scratch() {
    replay_scratch("tech_fd_marth");
}
#[test]
fn smash_death_and_revival_match_retail_scratch() {
    replay_scratch_until("ko_fd_marth", 480);
}
fn replay_scratch(name: &str) {
    replay_scratch_until(name, 300);
}
fn replay_scratch_until(name: &str, ticks: usize) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let path = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([path.clone()]),
    ) {
        return;
    }
    let mut simulation = super::TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        crate::trace::pad_script(&scenario).unwrap(),
    );
    // Fighter raw dumps stop at the fighter allocation; XRotN is a pointed-to
    // JObj. The companion retail bone recording carries its rotation instead.
    let mut roll_bones = if name.starts_with("damage_fly_roll_") {
        let bones = scenario.trace_path("bones.jsonl");
        if !melee_test_support::require_files([bones.clone()]) {
            return;
        }
        Some(melee_test_support::trace::open(&bones).unwrap().lines())
    } else {
        None
    };
    let raw = melee_test_support::trace::read_to_string(&path).unwrap();
    assert_eq!(raw.lines().count(), scenario.frames as usize);
    for (tick, line) in raw.lines().take(ticks).enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let bones: Option<serde_json::Value> = roll_bones.as_mut().map(|lines| {
            serde_json::from_str(&lines.next().expect("retail bone tick").unwrap()).unwrap()
        });
        if let Some(bones) = &bones {
            assert_eq!(bones["frame"].as_u64(), Some(tick as u64));
        }
        simulation.tick().unwrap();
        let runtime = &simulation.runtime;
        for slot in 0..2 {
            let bytes: Vec<u8> = row["fighters"][slot]["bytes"]
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(bytes[12], slot as u8, "slot order tick {tick}");
            crate::scene_fighter::with_fighter!(&runtime.state.fighters[slot], |f| {
                if name.starts_with("damage_fly_roll_") {
                    assert_eq!(
                        f.status.in_hitstun,
                        bytes[0x221C] & 2 != 0,
                        "hitstun owner tick {tick} slot {slot}"
                    );
                    if let MotionData::Damage(damage) = &f.state_data {
                        assert_eq!(
                            damage.trail_timer,
                            word(&bytes, 0x2348),
                            "damage trail timer tick {tick} slot {slot}"
                        );
                        assert_eq!(
                            damage.jump_buffer.to_bits(),
                            word(&bytes, 0x2354),
                            "damage jump buffer tick {tick} slot {slot}"
                        );
                    }
                    if let Some(bones) = &bones {
                        let bone = usize::from(
                            runtime.state.assets.fighters[slot]
                                .parts
                                .joint(melee_types::FtPart::XRotN)
                                .expect("XRotN"),
                        );
                        let joint = f.animation.parts[bone].joint;
                        let key = format!("p{slot}.bone[{bone}].rotate[0]");
                        let expected = bones["state"][&key]["v"]["bits"].as_u64().unwrap() as u32;
                        assert_eq!(
                            f.skeleton.rotation_x(joint).to_bits(),
                            expected,
                            "XRotN rotation tick {tick} slot {slot}"
                        );
                    }
                }
                if AIR_RELEASE_SCENARIOS.contains(&name) {
                    compare_wall_stop(f, &bytes, tick);
                    compare_capture_revival(f, &bytes, tick);
                    match &f.state_data {
                        MotionData::Capture(capture) => assert_eq!(
                            capture.jump_requested,
                            bytes[0x234c] != 0,
                            "capture jump latch tick {tick}"
                        ),
                        MotionData::CaptureJump(jump) => {
                            assert_eq!(
                                jump.frames.to_bits(),
                                word(&bytes, 0x2340),
                                "capture jump elapsed tick {tick}"
                            );
                            assert_eq!(
                                jump.retained_drop_timer.to_bits(),
                                word(&bytes, 0x2344),
                                "capture jump retained scratch tick {tick}"
                            );
                        }
                        _ => {}
                    }
                }
                if WALL_STOP_SCENARIOS.contains(&name) {
                    compare_wall_stop(f, &bytes, tick);
                }
                if name.starts_with("ledge_cstick_") || name.starts_with("ledge_timeout_") {
                    compare_ledge_input(f, &bytes, tick);
                }
                if name.starts_with("rebirth_")
                    || name.starts_with("grab_airborne_")
                    || name == "revival_laser_fd_marth_candidate"
                {
                    compare_capture_revival(f, &bytes, tick);
                }
                if name == "revival_laser_fd_marth_candidate" && slot == 1 {
                    // ftColl_80077C60 does not call plStale for an invincible receiver.
                    assert_eq!(
                        f.combat
                            .stale
                            .multiplier_for(
                                Some(melee_types::combat::StaleMove::SpecialNeutral),
                                &runtime.state.assets.fighters[slot].stale_weights,
                            )
                            .to_bits(),
                        1.0_f32.to_bits(),
                        "invincible laser must not stale tick {tick}"
                    );
                }
                if slot == 1
                    && (name.starts_with("firefox_") || name == "illusion_start_landing_fd_fox")
                {
                    compare_recovery_state(f, &bytes, tick);
                }
                if name.starts_with("cstick_throw_") {
                    compare_throw_state(f, &bytes, slot);
                }
                if name.starts_with("clank_") || name == "laser_reflect_overflow_air_timed_fd_marth"
                {
                    assert_eq!(
                        f.status.in_hitstun,
                        bytes[0x221C] & 2 != 0,
                        "hitstun owner tick {tick} slot {slot}"
                    );
                    assert_eq!(
                        f.commands.texture_animation_active,
                        bytes[0x221E] & 1 != 0,
                        "material owner tick {tick} slot {slot}"
                    );
                    if slot == 1 {
                        for group in [3, 4] {
                            let part = &f.animation.part_animations[group];
                            let offset = 0x8B0 + group * 0x14;
                            assert_eq!(
                                part.current,
                                bytes[offset + 17] as i8,
                                "part {group} selection tick {tick}"
                            );
                            assert_eq!(
                                part.previous,
                                bytes[offset + 16] as i8,
                                "part {group} prior selection tick {tick}"
                            );
                            if part.current != -1 {
                                assert_eq!(
                                    part.duration.to_bits(),
                                    word(&bytes, offset + 4),
                                    "part duration tick {tick}"
                                );
                                assert_eq!(
                                    part.progress.to_bits(),
                                    word(&bytes, offset + 8),
                                    "part progress tick {tick}"
                                );
                                assert_eq!(
                                    part.rate.to_bits(),
                                    word(&bytes, offset + 12),
                                    "part rate tick {tick}"
                                );
                            }
                        }
                    }
                }
                {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        compare(f, &bytes)
                    }));
                    assert!(
                        result.is_ok(),
                        "{name} tick {tick} slot {slot} action {}",
                        f.motion_state.action.0
                    );
                }
                if slot == 1 && REFLECTOR_INPUT_SCENARIOS.contains(&name) {
                    assert_eq!(f.physics.jumps_used, bytes[0x1968], "jump count at {tick}");
                    if (360..=369).contains(&f.motion_state.action.0) {
                        let reflector = &f.character.get::<ft_fox::init::Fox>().special_lw;
                        assert_eq!(
                            reflector.release_lag as u32,
                            word(&bytes, 0x2340),
                            "release lag at {tick}"
                        );
                        assert_eq!(
                            reflector.released,
                            word(&bytes, 0x2348) != 0,
                            "release latch at {tick}"
                        );
                        assert_eq!(
                            reflector.gravity_delay as u32,
                            word(&bytes, 0x234C),
                            "gravity delay at {tick}"
                        );
                        assert_eq!(
                            reflector.reflector.is_some(),
                            bytes[0x2218] & 0x10 != 0,
                            "reflection active at {tick}"
                        );
                        // +2344 holds the inherited word until Turn writes it.
                        if let Some(turn_frames) = reflector.turn_frames {
                            assert_eq!(
                                turn_frames as u32,
                                word(&bytes, 0x2344),
                                "turn countdown or inherited word at {tick}"
                            );
                        }
                        if matches!(f.motion_state.action.0, 364 | 369) {
                            assert_eq!(
                                f.commands.variables[0],
                                word(&bytes, 0x2200),
                                "turn latch at {tick}"
                            );
                        }
                    }
                }
                if slot == 0 && name.starts_with("aircounter_") {
                    assert_eq!(
                        f.physics.jumps_used, bytes[0x1968],
                        "Counter jump count at {tick}"
                    );
                    assert_eq!(
                        f.collision.lock_frames as u32,
                        word(&bytes, 0x88C),
                        "Counter ECB lock at {tick}"
                    );
                    if (369..=372).contains(&f.motion_state.action.0) {
                        let counter = &f.character.get::<ft_mars::init::Marth>().special_lw;
                        assert_eq!(
                            counter.damage as u32,
                            word(&bytes, 0x2340),
                            "Counter damage at {tick}"
                        );
                        assert_eq!(
                            f.commands.variables[1],
                            word(&bytes, 0x2204),
                            "Counter window at {tick}"
                        );
                        assert_eq!(
                            counter.volume.is_some(),
                            bytes[0x221B] & 0x80 != 0,
                            "Counter volume at {tick}"
                        );
                    }
                }
            });
        }
    }
    if let Some(mut bones) = roll_bones {
        assert!(bones.next().is_none(), "all retail bone ticks replayed");
    }
}

// S6: supplement canonical gates with retail shield health, analog amount,
// both pushback owners, hitlag and animation-rate words on every tick.
#[test]
fn shieldstun_ftilt_matches_retail_scratch() {
    replay_scratch("shieldstun_ftilt_fd_marth");
}

#[test]
fn laser_shield_matches_retail_health_stun_and_hitlag() {
    replay_scratch("laser_shield_fd_marth");
    replay_scratch("laser_lightshield_fd_marth");
    replay_scratch("laser_shield_air_fd_marth");
    replay_scratch("laser_shield_deflect_fd_marth");
}

#[test]
fn shieldtilt_ftilt_matches_retail_scratch() {
    replay_scratch("shieldtilt_ftilt_fd_marth");
}

#[test]
fn lightshield_ftilt_matches_retail_scratch() {
    replay_scratch("lightshield_ftilt_fd_marth");
}

#[test]
fn powershield_ftilt_matches_retail_scratch() {
    replay_scratch("powershield_ftilt_fd_marth");
}

/// shieldbreak_fd_marth.tick.raw.jsonl, ticks 220/257/283/313:
/// depletion resets health to PlCo+280, launches with intangibility, then
/// retains it through Down/Stand and clears it at dizzy entry.
#[test]
fn shield_exhaustion_and_dizzy_match_retail_milestones() {
    use melee_ft::fighter::MotionData;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario =
        Scenario::load(&root.join("harness/scenarios/shieldbreak_fd_marth.toml")).unwrap();
    let raw_path = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([raw_path.clone()]),
    ) {
        return;
    }
    let mut sim = super::TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        crate::trace::pad_script(&scenario).unwrap(),
    );
    let raw = melee_test_support::trace::read_to_string(&raw_path).unwrap();
    let milestones = [(220, 205), (257, 207), (283, 209), (313, 211), (519, 211)];
    for (tick, line) in raw.lines().enumerate() {
        sim.tick().unwrap();
        if let Some(&(_, state)) = milestones.iter().find(|&&(t, _)| t == tick) {
            let f = &sim.runtime.state.fighters[1];
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            let bytes: Vec<u8> = row["fighters"][1]["bytes"]
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(f.motion_state.action.0, state, "tick {tick}");
            compare(f, &bytes);
            assert_eq!(
                f.commands.hurt_status as u32,
                word(&bytes, 0x1988),
                "tick {tick} intangibility"
            );
            assert!(!f.shield.enabled);
            assert!(
                !f.status.unconditional_top_exit,
                "Fox uses the ordinary top-exit test"
            );
            if tick == 220 {
                assert_eq!(f.status.shield_health.to_bits(), 30.0_f32.to_bits());
            }
            if let MotionData::Dizzy(dizzy) = &f.state_data {
                assert_eq!(
                    dizzy.remaining.to_bits(),
                    word(&bytes, 0x1a4c),
                    "tick {tick} dizzy timer"
                );
            }
        }
    }
}

#[test]
fn aerial_counter_hit_and_landing_match_retail_scratch() {
    replay_scratch("aircounter_fd_marth");
    replay_scratch("aircounter_landing_fd_marth");
    replay_scratch("aircounter_hit_fd_marth");
    replay_scratch("aircounter_fall_fd_marth");
}

#[test]
fn reflector_input_matches_retail_scratch() {
    for name in REFLECTOR_INPUT_SCENARIOS {
        replay_scratch(name);
    }
}

#[test]
fn reflector_turn_root_rotation_matches_retail_bits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, ticks) in [
        ("reflectorturn_fd_fox", 80),
        ("airreflectorturn_fd_fox", 80),
        ("airreflectorturn_landing_fd_fox", 110),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let path = scenario.trace_path("bones.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([path.clone()]),
        ) {
            return;
        }
        let mut simulation = super::TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            crate::trace::pad_script(&scenario).unwrap(),
        );
        let bones = melee_test_support::trace::read_to_string(&path).unwrap();
        assert_eq!(bones.lines().count(), ticks);
        for (tick, line) in bones.lines().enumerate() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(row["frame"].as_u64(), Some(tick as u64));
            simulation.tick().unwrap();
            let expected = row["state"]["p1.bone[0].rotate[1]"]["v"]["bits"]
                .as_u64()
                .unwrap() as u32;
            crate::scene_fighter::with_fighter!(&simulation.runtime.state.fighters[1], |fighter| {
                let rotation = fighter.skeleton.rotation_y(fighter.animation.root);
                assert_eq!(rotation.to_bits(), expected, "{name} root Y at {tick}");
            });
        }
    }
}

// These fields distinguish article callbacks and linked hitlag from ordinary
// throw motion/percent output. The M5 gates separately compare live item state.
fn compare_throw_state(f: &Fighter, bytes: &[u8], slot: usize) {
    assert_eq!(
        f.commands.throw_accessory,
        bytes[0x2210] & 0x80 != 0,
        "throw accessory latch"
    );
    assert_eq!(
        f.commands.grab_release,
        bytes[0x2210] & 0x10 != 0,
        "throw release latch"
    );
    assert_eq!(
        f.commands.throw_reverse,
        bytes[0x2210] & 0x08 != 0,
        "throw reversal latch"
    );
    assert_eq!(
        f.combat.hitlag_remaining.to_bits(),
        word(bytes, 0x195c),
        "linked hitlag"
    );
    for (index, value) in f.commands.variables.iter().enumerate() {
        assert_eq!(
            *value,
            word(bytes, 0x2200 + index * 4),
            "command variable {index}"
        );
    }
    if slot == 1 && (219..=222).contains(&f.motion_state.action.0) {
        assert_eq!(
            f.character
                .get::<ft_fox::init::Fox>()
                .special_neutral
                .blaster_present,
            word(bytes, 0x222c) != 0,
            "throw blaster lifetime"
        );
    }
}

#[test]
fn cstick_throw_flags_articles_and_linked_hitlag_match_retail() {
    for character in ["fox", "marth"] {
        for direction in [
            "back",
            "down",
            "down_pulse",
            "forward",
            "horizontal_priority",
            "main_priority",
            "up",
        ] {
            replay_scratch(&format!("cstick_throw_{direction}_fd_{character}"));
        }
    }
}

#[test]
fn recovery_collisions_match_retail_combat_scratch() {
    for name in [
        "illusion_start_landing_fd_fox",
        "firefox_charge_landing_fd_fox",
        "firefox_ground_launch_fd_fox",
        "firefox_floor_rebound_fd_fox",
        "firefox_end_air_landing_fd_fox",
    ] {
        replay_scratch(name);
    }
}

/// Move-union words are compared only after their retail initialization.
fn compare_recovery_state(f: &Fighter, bytes: &[u8], tick: usize) {
    assert_eq!(f.physics.jumps_used, bytes[0x1968], "jumps at {tick}");
    for (index, value) in f.commands.variables.iter().enumerate() {
        assert_eq!(
            *value,
            word(bytes, 0x2200 + index * 4),
            "command {index} at {tick}"
        );
    }
    let fox = f.character.get::<ft_fox::init::Fox>();
    let action = f.motion_state.action.0;
    if (347..=352).contains(&action) {
        assert_eq!(
            fox.special_side.gravity_delay as u32,
            word(bytes, 0x2340),
            "Illusion gravity at {tick}"
        );
    }
    if (353..=359).contains(&action) {
        let hi = &fox.special_hi;
        assert_eq!(
            hi.gravity_delay as u32,
            word(bytes, 0x2340),
            "Fire Fox gravity at {tick}"
        );
        if action >= 355 {
            for (actual, offset) in [
                (hi.angle.to_bits(), 0x2344),
                (hi.travel_frames as u32, 0x2348),
                (hi.travel_ticks as u32, 0x234c),
                (hi.collision_ticks as u32, 0x2350),
            ] {
                assert_eq!(actual, word(bytes, offset), "Fire Fox {offset:x} at {tick}");
            }
        }
    }
}

#[test]
fn common_input_matches_retail_combat_scratch() {
    replay_scratch("run_shield_fd_fox");
    replay_scratch("run_shield_fd_marth");
    replay_scratch("run_taunt_fd_fox");
    replay_scratch("run_taunt_fd_marth");
    replay_scratch("run_shield_grab_fd_fox");
    replay_scratch("run_shield_grab_fd_marth");
    replay_scratch("taunt_fd_fox");
    replay_scratch("taunt_fd_marth");
    replay_scratch("dash_escape_fd_fox");
    replay_scratch("dash_escape_fd_marth");
    replay_scratch("dash_shield_fd_fox");
    replay_scratch("dash_shield_fd_marth");
    replay_scratch("dash_taunt_fd_fox");
    replay_scratch("dash_taunt_fd_marth");
    replay_scratch("dash_late_shield_fd_fox");
    replay_scratch("dash_late_shield_fd_marth");
    replay_scratch("shield_grab_fd_fox");
    replay_scratch("shield_grab_fd_marth");
    replay_scratch("dash_late_shield_grab_fd_fox");
    replay_scratch("dash_late_shield_grab_fd_marth");
    replay_scratch("shield_cstick_jump_fd_fox");
    replay_scratch("shield_cstick_jump_fd_marth");
    replay_scratch("shield_delayed_power_fd_fox");
    replay_scratch("shield_delayed_power_fd_marth");
}

fn compare_capture_revival(f: &Fighter, bytes: &[u8], tick: usize) {
    use melee_ft::fighter::{grab::GrabLink, life::LifeState};
    assert_eq!(
        f.status.revival_invincibility as u32,
        word(bytes, 0x1994),
        "revival protection at {tick}"
    );
    assert_eq!(
        f.collision.lock_frames as u32,
        word(bytes, 0x88c),
        "ECB lock at {tick}"
    );
    vector(f.physics.self_velocity, bytes, 0x80);
    vector(f.physics.knockback_velocity, bytes, 0x8c);
    match &f.state_data {
        MotionData::Life(
            LifeState::Revival { remaining, .. } | LifeState::PlatformWait { remaining, .. },
        ) => {
            assert_eq!(
                *remaining as u32,
                word(bytes, 0x2340),
                "revival countdown at {tick}"
            );
        }
        MotionData::Capture(capture) => {
            assert_eq!(
                capture.timer.to_bits(),
                word(bytes, 0x1a4c),
                "grab timer at {tick}"
            );
            assert_eq!(
                capture.elapsed.to_bits(),
                word(bytes, 0x2340),
                "capture elapsed at {tick}"
            );
            assert!(
                !capture.map_prepared,
                "paired capture Map consumed at {tick}"
            );
        }
        _ => {}
    }
    if let Some(GrabLink::Holding {
        vertical_offset, ..
    }) = f.combat.grab
    {
        assert_eq!(
            vertical_offset.to_bits(),
            word(bytes, 0x2170),
            "captor vertical offset at {tick}"
        );
    }
}

#[test]
fn capture_revival_matches_retail_scratch() {
    for (name, frames) in [
        ("rebirth_timeout_fd_fox", 900),
        ("rebirth_timeout_fd_marth", 900),
        ("rebirth_shield_a_fd_fox", 600),
        ("rebirth_analog_shield_a_fd_fox", 600),
        ("rebirth_held_shield_a_fd_fox", 600),
        ("grab_airborne_fd_foxmarth", 600),
        ("grab_airborne_fd_marthfox", 600),
    ] {
        replay_scratch_until(name, frames);
    }
}

fn compare_ledge_input(f: &Fighter, bytes: &[u8], tick: usize) {
    assert_eq!(
        f.status.ledge_cooldown as u32,
        word(bytes, 0x2064),
        "ledge cooldown at {tick}"
    );
    assert_eq!(
        f.status.ledge_intangibility as u32,
        word(bytes, 0x1990),
        "ledge intangibility at {tick}"
    );
    assert_eq!(
        f.status.ledge_timed_out,
        bytes[0x2227] & 0x40 != 0,
        "ledge timeout provenance at {tick}"
    );
    assert_eq!(
        f.status.on_ledge,
        bytes[0x221d] & 1 != 0,
        "on ledge at {tick}"
    );
    assert_eq!(
        f.status.grab_exclusions.0,
        u16::from_be_bytes([bytes[0x1a6a], bytes[0x1a6b]]),
        "grab exclusion at {tick}"
    );
    if let MotionData::Cliff(cliff) = &f.state_data {
        assert_eq!(
            cliff.ledge_id as u32,
            word(bytes, 0x2340),
            "ledge ID at {tick}"
        );
        // CliffCatch owns only the ledge ID. ftCo_8009A804 initializes the
        // wait timer and neutral latch when CliffWait begins; before then these
        // retail union words still belong to the preceding motion.
        if f.motion_state.id != melee_types::CommonMotionState::CliffCatch {
            assert_eq!(
                cliff.wait_frames.to_bits(),
                word(bytes, 0x2344),
                "ledge wait at {tick}"
            );
            assert_eq!(
                cliff.neutral_seen,
                word(bytes, 0x2348) != 0,
                "ledge neutral latch at {tick}"
            );
        }
    }
    if f.motion_state.id == melee_types::CommonMotionState::CliffWait {
        assert_eq!(
            bytes[0x2221] & 0x40,
            0,
            "uninterrupted ledge has no temporary part-status owner at {tick}"
        );
    }
}

#[test]
fn ledge_input_matches_retail_scratch() {
    for kind in ["fox", "marth"] {
        for option in ["attack", "escape", "drop", "priority"] {
            replay_scratch_until(&format!("ledge_cstick_{option}_fd_{kind}"), 420);
        }
        replay_scratch_until(&format!("ledge_timeout_fd_{kind}"), 1200);
    }
}

#[test]
fn fire_contact_matches_retail_damage_and_recovery_scratch() {
    replay_scratch("firefox_charge_hit_fd_marth");
    replay_scratch("firefox_travel_hit_fd_marth");
}

#[test]
fn laser_reflection_fresh_combat_scratch() {
    replay_scratch("laser_reflect_fresh_fd_marth");
}

#[test]
fn laser_reflection_return_combat_scratch() {
    replay_scratch("laser_reflect_return_boundary_fd_marth");
}

#[test]
fn laser_reflection_stale_combat_scratch() {
    replay_scratch("laser_reflect_stale_fd_marth");
}

#[test]
fn laser_reflection_delayed_combat_scratch() {
    replay_scratch("laser_reflect_delayed_timed_fd_marth");
}

#[test]
fn contact_closure_mutual() {
    replay_scratch_until("clank_jab_s74_f122_fd_foxmarth", 300);
}

#[test]
fn contact_closure_priority_fox() {
    replay_scratch_until("clank_priority_fox_spaced_fd_foxmarth", 300);
}

#[test]
fn contact_closure_priority_marth() {
    replay_scratch_until("clank_priority_marth_spaced_fd_foxmarth", 300);
}

#[test]
fn contact_closure_no_rebound() {
    replay_scratch_until("clank_smash_norebound_spaced_fd_foxmarth", 300);
}

#[test]
fn contact_closure_airborne_fox() {
    replay_scratch_until("clank_airborne_fox_spaced_fd_foxmarth", 300);
}

#[test]
fn contact_closure_airborne_marth() {
    replay_scratch_until("clank_airborne_marth_spaced_fd_foxmarth", 300);
}

#[test]
fn contact_closure_overflow() {
    replay_scratch_until("laser_reflect_overflow_air_timed_fd_marth", 600);
}

const WALL_STOP_SCENARIOS: [&str; 4] = [
    "walljump_right_underside_fd_fox_candidate",
    "stopceil_latejump266_fd_fox_candidate",
    "stopceil_left_latejump270_fd_marth_candidate",
    "walljump_left_underside_fd_marth_control",
];

#[test]
fn walljump_stopceil_matches_retail_owned_scratch() {
    for name in WALL_STOP_SCENARIOS {
        replay_scratch_until(name, 450);
    }
}

fn compare_wall_stop(f: &Fighter, bytes: &[u8], tick: usize) {
    use melee_types::CommonMotionState as S;
    assert_eq!(
        u32::from(f.motion_state.action.0),
        word(bytes, 0x10),
        "action tick {tick}"
    );
    for (actual, offset) in [(f.animation.frame, 0x894), (f.animation.speed, 0x89c)] {
        assert_eq!(
            actual.to_bits(),
            word(bytes, offset),
            "clock {offset:x} tick {tick}"
        );
    }
    // Persistent fields are restored from the initial boundary for BOTH kinds.
    // Ineligible Marth must leave 210C/2110 unchanged, not execute Fox's predicate.
    assert_eq!(
        f.status.wall_jump.used, bytes[0x1969],
        "walljump count tick {tick}"
    );
    assert_eq!(
        f.status.wall_jump.contact_age, bytes[0x210c],
        "wall contact age tick {tick}"
    );
    assert_eq!(
        f.status.wall_jump.side.to_bits(),
        word(bytes, 0x2110),
        "wall side tick {tick}"
    );
    vector(f.physics.position, bytes, 0xb0);
    vector(f.physics.previous_position, bytes, 0xbc);
    vector(f.physics.position_delta, bytes, 0xc8);
    vector(f.physics.self_velocity, bytes, 0x80);
    assert_eq!(
        f.collision.lock_frames as u32,
        word(bytes, 0x88c),
        "ECB lock tick {tick}"
    );
    let cd = &f.collision.data;
    for (value, offset) in [
        (cd.cur_pos, 0x6f4),
        (cd.prev_pos, 0x700),
        (cd.last_pos, 0x70c),
    ] {
        vector(value, bytes, offset);
    }
    for (value, offset) in [
        (cd.x130_flags, 0x820),
        (cd.env_flags as u32, 0x824),
        (cd.prev_env_flags as u32, 0x828),
        (cd.floor_skip as u32, 0x72c),
    ] {
        assert_eq!(
            value,
            word(bytes, offset),
            "collision flags {offset:x} tick {tick}"
        );
    }
    let pack = |v: &melee_types::mp::EcbFlags| {
        (u8::from(v.b0) << 7)
            | (v.b1234 << 3)
            | (u8::from(v.b5) << 2)
            | (u8::from(v.b6) << 1)
            | u8::from(v.b7)
    };
    assert_eq!(pack(&cd.x34_flags), bytes[0x724], "ECB flags34 tick {tick}");
    assert_eq!(pack(&cd.x35_flags), bytes[0x725], "ECB flags35 tick {tick}");
    for (ecb, offset) in [
        (&cd.x64_ecb, 0x754),
        (&cd.desired_ecb, 0x774),
        (&cd.ecb, 0x794),
        (&cd.prev_ecb, 0x7b4),
        (&cd.xe4_ecb, 0x7d4),
    ] {
        for (index, point) in [ecb.top, ecb.bottom, ecb.right, ecb.left]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                point.x.to_bits(),
                word(bytes, offset + index * 8),
                "ECB x {offset:x}/{index} tick {tick}"
            );
            assert_eq!(
                point.y.to_bits(),
                word(bytes, offset + index * 8 + 4),
                "ECB y {offset:x}/{index} tick {tick}"
            );
        }
    }
    for (surface, offset) in [
        (&cd.floor, 0x83c),
        (&cd.left_facing_wall, 0x850),
        (&cd.right_facing_wall, 0x864),
        (&cd.ceiling, 0x878),
    ] {
        assert_eq!(
            surface.index as u32,
            word(bytes, offset),
            "surface {offset:x} tick {tick}"
        );
        // A missing surface has no active normal/material owner. Its stale bytes
        // are not an assertion of the next contact's initialized surface data.
        if surface.index >= 0 {
            assert_eq!(
                surface.flags,
                word(bytes, offset + 4),
                "surface flags {offset:x} tick {tick}"
            );
            vector(surface.normal, bytes, offset + 8);
        }
    }
    let owned = matches!(f.motion_state.id, S::PassiveWallJump | S::StopCeil);
    if let MotionData::WallJump(state) = &f.state_data {
        for (actual, offset) in [
            (state.freeze_frames as u32, 0x2340),
            (state.retained_zero as u32, 0x2344),
            (u32::from(state.jump_buffered), 0x2348),
            (state.vertical_exponent as u32, 0x234c),
        ] {
            assert_eq!(
                actual,
                word(bytes, offset),
                "walljump scratch {offset:x} tick {tick}"
            );
        }
    }
    if owned {
        if f.motion_state.id == S::PassiveWallJump {
            // ftCo_800C1E64 initializes cmd0 only; StopCeil owns no cmd vars.
            assert_eq!(
                f.commands.variables[0],
                word(bytes, 0x2200),
                "walljump cmd0 tick {tick}"
            );
        }
        for (value, mask) in [
            (f.commands.throw_accessory, 0x80),
            (f.commands.throw_reverse, 0x40),
            (f.commands.grab_release, 0x10),
        ] {
            assert_eq!(
                value,
                bytes[0x2210] & mask != 0,
                "throw flags {mask:x} tick {tick}"
            );
        }
        assert_eq!(
            f.commands.texture_animation_active,
            bytes[0x221e] & 1 != 0,
            "material owner tick {tick}"
        );
        assert_eq!(
            f.status.ledge_cooldown as u32,
            word(bytes, 0x2064),
            "ledge cooldown tick {tick}"
        );
        let root = f
            .animation
            .root_motion
            .as_ref()
            .expect("wall/ceiling TransN owner");
        for (value, offset) in [
            (root.primary_history.position, 0x68c),
            (root.primary_history.previous, 0x698),
            (root.primary_history.offset, 0x6a4),
            (root.primary_history.previous_offset, 0x6b0),
        ] {
            vector(value, bytes, offset);
        }
        if f.animation
            .flags
            .contains(melee_ft::anim::playback::MotionFlags::SECOND_ROOT)
        {
            for (value, offset) in [
                (root.secondary_history.position, 0x6c0),
                (root.secondary_history.previous, 0x6cc),
                (root.secondary_history.offset, 0x6d8),
                (root.secondary_history.previous_offset, 0x6e4),
            ] {
                vector(value, bytes, offset);
            }
        }
    }
}

const AIR_RELEASE_SCENARIOS: [&str; 6] = [
    "capture_jump_up_release_fd_foxmarth_candidate",
    "capture_jump_up_release_fd_marthfox_candidate",
    "capture_jump_xy_latch_fd_foxmarth_candidate",
    "capture_jump_xy_latch_fd_marthfox_candidate",
    "capture_edge_fox_outward_stop43_jump109_grab107_candidate",
    "capture_edge_fox_air_up_release_candidate",
];

#[test]
fn air_capture_release_matches_owned_scratch() {
    for name in AIR_RELEASE_SCENARIOS {
        replay_scratch_until(name, 450);
    }
}

#[test]
fn revival_laser_invincibility() {
    replay_scratch_until("revival_laser_fd_marth_candidate", 600);
}

#[test]
fn damage_fly_roll_t125_scratch() {
    replay_scratch_until("damage_fly_roll_t125_fd_fox_candidate", 450);
}

#[test]
fn damage_fly_roll_dtilt_t132_scratch() {
    replay_scratch_until("damage_fly_roll_dtilt_t132_fd_fox_candidate", 450);
}

#[test]
fn damage_fly_roll_crouch_scratch() {
    replay_scratch_until("damage_fly_roll_crouch_fd_fox_candidate", 450);
}
