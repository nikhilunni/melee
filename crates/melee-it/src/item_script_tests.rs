//! Running item programs keep their identity when only the motion changes.
use super::*;

struct Inert;
impl ItemLogic for Inert {
    const KIND: ItemKind = ItemKind::FoxBlaster;
    const STATES: &'static [ItemStateRow] = &[];
}
item_kinds! { enum Kinds { Inert: Inert } }

fn item(assets: &ItemAssets) -> ItemCore {
    let _kind = Kinds::Inert;
    let mut pool = ItemPool::new(ItemCommonData {
        random_items: Default::default(),
        hold_limits: [None; 13],
        lifetime: 1.0,
        spawn_intangible_frames: 0.0,
        half_life_scale: 0.5,
        speed_damage_scale: 0.0,
        speed_damage_base: 0.0,
        shield_bounce_degrees: 0.0,
        maximum_reflected_damage: 999,
        clank_priority_gap: 9,
        hitlag_scale: 0.0,
        hitlag_base: 0.0,
        explosion_lifetime: 0.0,
        spawn_spin_degrees: 0.0,
        fall_spin_degrees: 0.0,
        knockback: Default::default(),
        launch: Default::default(),
        victim_bounce: Default::default(),
        camera_extents: [0.0; 4],
        push_speed: 0.0,
        release_floor_depth: 0.0,
        direction_still_speed: 0.0,
        slide_rule: crate::desc::SlideRule::NEVER,
    });
    let id = pool
        .spawn::<Kinds>(
            SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0),
            assets,
            &mut hsd_gobj::GObjAddresses::default(),
        )
        .unwrap();
    pool.get_mut(id).unwrap().clone()
}

#[test]
fn delayed_continuation_uses_its_original_program_after_a_motion_change() {
    let mut assets = tests::assets();
    assets.scripts = vec![
        vec![
            Command::Call {
                target: 4,
                continuation: 1,
            },
            Command::Wait(3.0),
            Command::SetVariable { index: 0, value: 7 },
            Command::End,
            Command::Return,
        ],
        vec![
            Command::Call {
                target: 2,
                continuation: 1,
            },
            Command::End,
            Command::Return,
        ],
    ];
    let mut item = item(&assets);
    item.change_motion_with(0, 0, state_change::ANIM_UPDATE, &assets);
    assert_eq!(item.script.instruction, Some(2));
    assert_eq!(item.script.timer, 2.0);
    item.change_motion_with(1, 1, state_change::HIT_PRESERVE, &assets);
    assert_eq!(item.script.timer, 1.0);
    item.advance_script(&assets);
    assert_eq!(item.command_variables[0], 7);
    assert_eq!(item.script.instruction, None);
    assert!(item.hitboxes.iter().all(Option::is_none));
}

#[test]
fn preserved_call_and_loop_stacks_survive_cloning_independently() {
    let mut assets = tests::assets();
    assets.scripts = vec![
        vec![
            Command::BeginLoop(2),
            Command::Call {
                target: 4,
                continuation: 2,
            },
            Command::EndLoop,
            Command::End,
            Command::Wait(3.0),
            Command::SetVariable { index: 1, value: 9 },
            Command::Return,
        ],
        vec![Command::End],
    ];
    let mut item = item(&assets);
    item.change_motion_with(0, 0, state_change::ANIM_UPDATE, &assets);
    item.change_motion_with(1, 1, state_change::HIT_PRESERVE, &assets);
    assert_eq!(item.script_source_motion, Some(0));
    assert_eq!(item.script.return_stack.len(), 1);
    assert_eq!(item.script.loops[0].remaining, 2);
    let mut branch = item.clone();
    item.advance_script(&assets);
    assert_eq!(item.command_variables[1], 9);
    assert_eq!(item.script.loops[0].remaining, 1);
    assert_eq!(branch.command_variables[1], 0);
    assert_eq!(branch.script.loops[0].remaining, 2);
    for _ in 0..3 {
        item.advance_script(&assets);
    }
    assert_eq!(item.script.instruction, None);
    assert!(item.script.return_stack.is_empty());
    assert!(item.script.loops.is_empty());
    for _ in 0..4 {
        branch.advance_script(&assets);
    }
    assert_eq!(branch.script.instruction, None);
    assert_eq!(branch.command_variables, item.command_variables);
}

#[test]
fn animation_and_command_resets_replace_the_source_and_pose_clear_removes_it() {
    let mut assets = tests::assets();
    assets.scripts = vec![
        vec![Command::Wait(30.0), Command::End],
        vec![Command::SetVariable { index: 0, value: 4 }, Command::End],
        vec![],
    ];
    for flags in [state_change::ANIM_UPDATE, state_change::CMD_UPDATE] {
        let mut item = item(&assets);
        item.change_motion(0, &assets);
        item.change_motion_with(1, 1, flags, &assets);
        assert_eq!(item.script_source_motion, Some(1));
        assert_eq!(item.command_variables[0], 4);
        assert_eq!(item.script.instruction, None);
        item.change_motion(0, &assets);
        item.change_motion_with(1, -1, 0, &assets);
        assert_eq!(item.script_source_motion, None);
        assert_eq!(item.script.instruction, None);
        item.change_motion(0, &assets);
        item.pose_article_frame(5.0);
        assert_eq!(item.script_source_motion, None);
        assert_eq!(item.script.instruction, None);
        item.change_motion(0, &assets);
        item.change_motion_with(2, 2, flags, &assets);
        assert_eq!(item.script_source_motion, None, "empty program");
    }
}

#[test]
fn preserved_hitbox_command_uses_the_same_programs_parallel_flags() {
    let mut assets = tests::assets();
    let descriptor = melee_types::combat::HitboxDescriptor {
        group: 0,
        bone: 0,
        common_bone: false,
        requires_throw_owner: false,
        damage: 5.0,
        shield_damage: 0,
        sound_severity: 0,
        radius: 2.0,
        offset: Vec3::ZERO,
        angle: 45,
        growth: 10,
        weight_knockback: 0,
        base_knockback: 1,
        element: melee_types::HitElement::Normal,
        hit_ground: true,
        hit_air: true,
        ignore_scale: false,
        clank: false,
        rebound: false,
    };
    let program = vec![
        Command::Wait(3.0),
        Command::SpawnHitbox { id: 0, descriptor },
        Command::End,
    ];
    assets.scripts = vec![program.clone(), program];
    let original = desc::ItemHitFlags {
        reflectable: true,
        auxiliary: 17,
        ..Default::default()
    };
    assets.hit_flags = vec![
        vec![None, Some(original), None],
        vec![
            None,
            Some(desc::ItemHitFlags {
                absorbable: true,
                auxiliary: 29,
                ..Default::default()
            }),
            None,
        ],
    ];
    let mut item = item(&assets);
    item.change_motion_with(0, 0, state_change::ANIM_UPDATE, &assets);
    item.change_motion_with(1, 1, state_change::HIT_PRESERVE, &assets);
    item.advance_script(&assets);
    assert_eq!(item.script_source_motion, Some(0));
    assert!(item.hitboxes[0].is_some());
    assert!(item.hit_flags[0].reflectable);
    assert!(!item.hit_flags[0].absorbable);
    assert_eq!(item.hit_flags[0].auxiliary, original.auxiliary);
}
