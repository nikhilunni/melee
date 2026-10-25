//! Refusal tests use a real ordinary item contact pose with a synthetic
//! pending log. They do not claim a retail mixed-contact witness.
use super::*;
use crate::scenario::Scenario;
use melee_coll::damage::ReceivedHit;
use melee_coll::damage_log::{HitCredit, HitSource, LoggedHit};
use melee_coll::hurtbox::HurtHeight;
use std::path::PathBuf;

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

#[test]
fn item_ordinary_early_returns_refuse_pending_phantoms_before_response_mutation() {
    let scenario = Scenario::load(
        &root().join("harness/scenarios/corpus_v3_fd_kirby_captainfalcon4_ee42e8dad_p0.toml"),
    )
    .unwrap();
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    // This complete capture's actual ordinary invincible CBeam contact.
    for frame in 0..=740 {
        simulation
            .engine
            .set_display_pass(simulation.pads.display_pass(frame));
        simulation.tick_without_snapshot().unwrap();
    }
    let state = simulation.state();
    let item = state
        .items
        .iter()
        .find(|item| item.kind == melee_types::ItemKind::KirbyCBeam)
        .unwrap();
    let mut ordinary = item.clone();
    for hit in ordinary.hitboxes.iter_mut().flatten() {
        hit.victims.clear();
        hit.phantom_victims = Default::default();
    }
    let victim = &state.fighters[1].0;
    let mut pose = (**victim).clone();
    let hit = ordinary.hitboxes[0].as_ref().unwrap();
    let (contact, _, _) =
        melee_coll::detection::first_hurt_contact(&mut pose.core, hit, ordinary.scale).unwrap();
    assert!(contact.overlap >= state.assets.fighters[1].damage.phantom_threshold);
    let damage = pose
        .clone()
        .detect_item_hit(
            &mut ordinary.clone(),
            &state.assets.fighters[1],
            state.items.common(),
            false,
        )
        .expect("actual ordinary contact")
        .damage;
    assert!(damage > 0.0);
    let entry = LoggedHit {
        source: HitSource::Fighter(123),
        credit: HitCredit::Player(0),
        hit: ReceivedHit {
            descriptor: hit.descriptor.clone(),
            height: HurtHeight::Middle,
            facing: 1.0,
            knockback: 0.0,
            facing_override: None,
            percent_damage: 1.0,
        },
        position: contact.position,
        knockback_damage: 1,
        damage: 1.0,
        effect_damage: 1.0,
        attack_ratio: 1.0,
    };
    // A resolved older phantom is independent of this frame's pending log.
    let old_phantom = melee_ft::fighter::damage::PhantomHit {
        position: contact.position,
        element: hit.descriptor.element,
        sound_severity: 0,
        source: entry.source,
        damage: 1.0,
    };
    // Move only this test capsule along x to find a genuine shallow
    // contact through the actual hurt geometry. Even a pending log and
    // intact shell must not send this phantom-only branch through refusal.
    let mut phantom_item = ordinary.clone();
    phantom_item.hitboxes[1..].fill(None);
    let original = phantom_item.hitboxes[0].as_ref().unwrap().clone();
    let mut found = false;
    for step in -20_000..=20_000 {
        let hit = phantom_item.hitboxes[0].as_mut().unwrap();
        hit.position.x = original.position.x + step as f32 * 0.001;
        hit.previous_position = hit.position;
        if let Some((contact, _, _)) =
            melee_coll::detection::first_hurt_contact(&mut pose.core, hit, phantom_item.scale)
        {
            if contact.overlap < state.assets.fighters[1].damage.phantom_threshold {
                found = true;
                break;
            }
        }
    }
    assert!(found, "actual geometric item phantom contact");
    let mut phantom_victim = pose.clone();
    phantom_victim.core.begin_hit_detection();
    phantom_victim.status.revival_invincibility = 0;
    phantom_victim.commands.hurt_status = melee_types::combat::HurtStatus::Normal;
    phantom_victim.set_hurt_capsules(melee_types::combat::HurtStatus::Normal);
    phantom_victim.combat.phantom_lockout = 0.0;
    phantom_victim.combat.shell_active = true;
    phantom_victim.combat.shell_health = 38.0;
    phantom_victim.combat.phantom_log.push(entry.clone());
    phantom_victim.commands.hitboxes.fill(None);
    assert!(phantom_victim
        .detect_item_hit(
            &mut phantom_item,
            &state.assets.fighters[1],
            state.items.common(),
            false,
        )
        .is_none());
    assert_eq!(phantom_victim.combat.phantom_log.len(), 2);
    assert_eq!(phantom_victim.combat.shell_health, 38.0);
    assert!(phantom_victim.combat.hit_log.is_empty());
    assert!(phantom_item.hitboxes[0]
        .as_ref()
        .unwrap()
        .phantom_victims
        .contains(phantom_victim.spawn_number));
    for (name, pending, invincible, shell, refused) in [
        (
            "pending invincible",
            true,
            true,
            None,
            Some("ordinary invincible"),
        ),
        (
            "pending held",
            true,
            false,
            Some(damage + 10.0),
            Some("ordinary shell-held"),
        ),
        (
            "pending exact zero",
            true,
            false,
            Some(damage),
            Some("ordinary shell-held"),
        ),
        (
            "pending unordered shell",
            true,
            false,
            Some(f32::from_bits(0x7fc0_1234)),
            Some("ordinary shell-held"),
        ),
        ("empty invincible", false, true, None, None),
        ("empty held", false, false, Some(damage + 10.0), None),
        ("pending ordinary damage", true, false, None, None),
        ("pending shell break", true, false, Some(damage - 1.0), None),
    ] {
        let mut victim = pose.clone();
        let mut item = ordinary.clone();
        victim.core.begin_hit_detection();
        victim.shield.active = false;
        victim.commands.hitboxes.fill(None);
        victim.commands.hurt_status = melee_types::combat::HurtStatus::Normal;
        victim.set_hurt_capsules(melee_types::combat::HurtStatus::Normal);
        victim.status.revival_invincibility = i32::from(invincible);
        victim.combat.shell_active = shell.is_some();
        victim.combat.shell_health = shell.unwrap_or(38.0);
        victim.combat.phantom = Some(old_phantom.clone());
        victim.combat.phantom_lockout = 4.0;
        if pending {
            victim.combat.phantom_log.push(entry.clone());
        }
        let before_effects = victim.effects.clone();
        let before = (
            victim.combat.shell_health.to_bits(),
            victim.combat.shell_active,
            format!("{:?}", victim.combat.phantom_log),
            format!("{:?}", item),
            victim.effects.len(),
            victim.combat.frame_damage.to_bits(),
            victim.combat.frame_max_damage,
        );
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            victim.detect_item_hit(
                &mut item,
                &state.assets.fighters[1],
                state.items.common(),
                false,
            )
        }));
        if let Some(message) = refused {
            let error = result.expect_err(name);
            let text = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(text.contains("ftColl_80077C60 (ftcoll.c:1233,"), "{text}");
            assert!(text.contains(message), "{text}");
            assert_eq!(
                before,
                (
                    victim.combat.shell_health.to_bits(),
                    victim.combat.shell_active,
                    format!("{:?}", victim.combat.phantom_log),
                    format!("{:?}", item),
                    victim.effects.len(),
                    victim.combat.frame_damage.to_bits(),
                    victim.combat.frame_max_damage,
                ),
                "{name}: unsupported ordinary response has not changed state"
            );
            assert!(victim.combat.hit_log.is_empty());
            assert_eq!(victim.effects, before_effects, "{name}: exact effect queue");
        } else {
            assert!(result.unwrap().is_some(), "{name}: real ordinary contact");
            assert!(victim.combat.phantom_log.is_empty(), "{name}");
            assert!(item.hitboxes[0]
                .as_ref()
                .unwrap()
                .victims
                .contains(&victim.spawn_number));
            #[allow(clippy::neg_cmp_op_on_partial_ord)] // Same unordered-inclusive predicate.
            let held = shell.is_some_and(|health| !(health - damage < 0.0));
            assert_eq!(
                victim.combat.hit_log.is_empty(),
                invincible || held,
                "{name}"
            );
            if !invincible {
                assert_eq!(victim.combat.shell_active, held);
                if let Some(health) = shell {
                    assert_eq!(
                        victim.combat.shell_health.to_bits(),
                        (health - damage).to_bits()
                    );
                }
            }
        }
        assert_eq!(victim.combat.phantom_lockout, 4.0, "{name}");
        assert_eq!(
            format!("{:?}", victim.combat.phantom),
            format!("{:?}", Some(&old_phantom))
        );
    }
}

#[test]
fn supported_item_contacts_keep_complete_raw_shell_and_phantom_state() {
    use std::io::BufRead;
    for (name, ticks) in [
        ("kirby_stone_lasers_fd_fox4", 460),
        ("kirby_stone_laser_break_fd_fox4", 520),
        ("corpus_v3_fd_kirby_captainfalcon4_ee42e8dad_p0", 6001),
        ("corpus_v3_ys_kirby_fox4_ea4d5d9ec_p1", 5542),
    ] {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames, ticks);
        let raw = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()]),
        ) {
            return;
        }
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut count = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            for (port, fighter) in simulation.state().fighters.iter().enumerate() {
                let hex = row["fighters"][port]["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                let f = &fighter.0;
                for (value, offset) in [
                    (f.combat.hitlag_remaining, 0x195c),
                    (f.combat.phantom_lockout, 0x189c),
                    (f.combat.phantom_knockback, 0x18a0),
                    (f.combat.shell_health, 0x1834),
                    (f.physics.position.x, 0xb0),
                    (f.physics.position.y, 0xb4),
                    (f.physics.position.z, 0xb8),
                ] {
                    assert_eq!(
                        value.to_bits(),
                        word(offset),
                        "{name} tick{frame} p{port} +{offset:x}"
                    );
                }
                let shell =
                    u8::from_str_radix(&hex[0x221c * 2..0x221c * 2 + 2], 16).unwrap() & 8 != 0;
                assert_eq!(
                    f.combat.shell_active, shell,
                    "{name} tick{frame} p{port} shell flag"
                );
            }
            count += 1;
        }
        assert_eq!(count, ticks as usize);
    }
}
