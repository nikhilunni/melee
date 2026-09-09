//! Fighter boundary adapter, promoted from melee-ft/tests/fighter_support.
use super::{float, vector, word};
use crate::assets::Assets;
use ft_fox::init::Fox;
use hsd_types::Vec3;
use melee_ft::{
    collision::pose::GroundPoseFlags,
    fighter::{Fighter, PlayerSlot, RetailTrig},
    input::{AnalogTimers, Buttons, FighterInput, InputFrame, Stick},
};
use melee_mp::CollMap;
use melee_types::PlayerKind;
/// Import only the saved boundary. No later row is used by this constructor.
pub(super) fn import(assets: &Assets, map: &CollMap, raw: &[u8]) -> Fighter<Fox> {
    let mut player = PlayerSlot {
        id: raw[12],
        control: PlayerKind::Human,
        costume: 0,
        stocks: 1,
        position: Vec3::ZERO,
        facing: 1.0,
        scale: 1.0,
        damage: 0.0,
        cpu_mode: 4,
        cpu_level: 1,
    };
    player.position = vector(raw, 0xB0);
    player.facing = float(raw, 0x2C);
    player.damage = float(raw, 0x1830);
    player.costume = raw[0x619];
    let (tree, root) = assets.model(player.costume);
    let mut f = Fighter::prepare(player, assets.character(), &assets.fighter, tree, root, map);
    let root = f.animation.root;
    if word(raw, 0x14) == u32::MAX {
        f.animation.clear_motion(&mut f.skeleton);
    } else {
        f.animation
            .set_animation(
                &mut f.skeleton,
                &assets.fighter.motions[&(word(raw, 0x14) as i32)],
                float(raw, 0x894),
                float(raw, 0x89C),
            )
            .unwrap();
        f.animation.blend_progress = float(raw, 0x8A8) - 1.0;
        f.animation.step::<RetailTrig>(&mut f.skeleton);
        f.animation.blend_progress = float(raw, 0x8A8);
        f.animation.remainder = float(raw, 0x898);
    }
    f.physics.self_velocity = vector(raw, 0x80);
    f.physics.knockback_velocity = vector(raw, 0x8C);
    f.physics.shield_knockback_velocity = vector(raw, 0x98);
    f.physics.animation_velocity = vector(raw, 0x74);
    f.physics.previous_position = vector(raw, 0xBC);
    f.physics.position_delta = vector(raw, 0xC8);
    f.physics.ground_velocity = float(raw, 0xEC);
    f.physics.ground_acceleration = float(raw, 0xE4);
    f.physics.secondary_ground_acceleration = float(raw, 0xE8);
    f.physics.ground_knockback_velocity = float(raw, 0xF0);
    f.physics.ground_shield_knockback_velocity = float(raw, 0xF4);
    f.physics.jumps_used = raw[0x1968];
    f.physics.fast_fall = raw[0x221A] & 8 != 0;
    f.physics.ground_or_air = melee_types::GroundOrAir::try_from(word(raw, 0xE0) as i32).unwrap();
    f.input = import_input(raw);
    f.cpu.buttons = word(raw, 0x1A88);
    f.cpu.stick = [raw[0x1A8C] as i8, raw[0x1A8D] as i8];
    f.cpu.mode = word(raw, 0x1A94) as i32;
    f.cpu.level = word(raw, 0x1A98) as i32;
    f.cpu.behavior = word(raw, 0x1AA0) as i32;
    f.cpu.reaction_timer = word(raw, 0x1B04) as i32;
    f.status.input_frozen = raw[0x221D] & 8 != 0;
    f.status.shield_health = float(raw, 0x1998);
    f.status.name_tag_timer = u16::from_be_bytes([raw[0x209A], raw[0x209B]]);
    f.status.ledge_cooldown = word(raw, 0x2064) as i32;
    f.status.ledge_intangibility = word(raw, 0x1990) as i32;
    f.status.on_ledge = raw[0x221D] & 1 != 0;
    f.status.grab_exclusions =
        melee_ft::fighter::ledge::GrabExclusions(u16::from_be_bytes([raw[0x1A6A], raw[0x1A6B]]));
    f.status.ledge_grab_disabled = raw[0x2228] & 0x20 != 0;
    f.thrown_hitbox.state = word(raw, 0x1064);
    f.thrown_hitbox.offset = vector(raw, 0x1074);
    f.thrown_hitbox.position = vector(raw, 0x10B0);
    f.thrown_hitbox.previous_position = vector(raw, 0x10BC);
    f.ground_pose =
        GroundPoseFlags(((u16::from_be_bytes([raw[0x221C], raw[0x221D]]) >> 6) & 7) as u8);
    f.dynamics_first_bone = (0..word(raw, 0x3E0) as usize)
        .map(|i| word(raw, 0x2F0 + i * 0x18))
        .collect();
    // Fighter.x8B0[5], stride 0x14 (ft/types.h:1301-1308).
    // ftAnim_800707B0 ignores a slot only when current is -1. Preserve
    // even inactive scalar words rather than infer defaults from a pose.
    for (index, state) in f.animation.part_animations.iter_mut().enumerate() {
        let offset = 0x8B0 + index * 0x14;
        state.state = word(raw, offset) as i32;
        state.duration = float(raw, offset + 4);
        state.progress = float(raw, offset + 8);
        state.rate = float(raw, offset + 12);
        state.previous = raw[offset + 16] as i8;
        state.current = raw[offset + 17] as i8;
        state.joints = f.bones.animation_sets[index]
            .as_ref()
            .map_or_else(Vec::new, |set| {
                set.joints.iter().map(|&joint| usize::from(joint)).collect()
            });
    }
    let archive_base = word(raw, 0x24) - assets.fighter.motion_table_offset;
    let pc = word(raw, 0x3EC).wrapping_sub(archive_base);
    f.commands.instruction = (word(raw, 0x3EC) != 0).then(|| {
        assets
            .fighter
            .instruction_offsets
            .iter()
            .position(|&p| p == pc)
            .unwrap()
    });
    f.commands.timer = float(raw, 0x3E4);
    f.commands.frame = float(raw, 0x3E8);
    f.commands.variables = std::array::from_fn(|i| word(raw, 0x2200 + i * 4));
    f.effect_state.destroy_on_state_change = raw[0x2219] & 0x80 != 0;
    f.effect_state.rotating_bone_index = raw[0x2220] >> 5;
    if f.commands.instruction.is_some() {
        assert_eq!(word(raw, 0x3F0), 0, "initial command return stack empty");
    } // SM_None clears the script pointer; its old union bytes are inactive.
    f.skeleton.set_rotation_y(
        root,
        (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32,
    );
    super::collision::restore(&mut f.collision, raw);
    if word(raw, 0x10) == 322 {
        use hsd_types::Vec2;
        use melee_ft::fighter::{entry::EntryState, MotionData, MotionState};
        f.motion_state = MotionState::ENTRY;
        let current_scale = vector(raw, 0x2354);
        f.skeleton.set_scale(root, &current_scale);
        f.state_data = MotionData::Entry(EntryState {
            timer: word(raw, 0x2340) as i32,
            origin_y: float(raw, 0x2344),
            original_scale: vector(raw, 0x2348),
            current_scale,
            trophy_height: float(raw, 0x2360),
            trophy_scale: float(raw, 0x2364),
            lift: float(raw, 0x2368),
            collision_box: melee_types::mp::FtCollisionBox {
                top: float(raw, 0x236C),
                bottom: float(raw, 0x2370),
                left: Vec2::new(float(raw, 0x2374), float(raw, 0x2378)),
                right: Vec2::new(float(raw, 0x237C), float(raw, 0x2380)),
            },
        });
    }

    f
}
fn import_input(raw: &[u8]) -> FighterInput {
    let frame = |i: usize| InputFrame {
        stick: Stick {
            x: float(raw, 0x620 + i * 8),
            y: float(raw, 0x624 + i * 8),
        },
        cstick: Stick {
            x: float(raw, 0x638 + i * 8),
            y: float(raw, 0x63C + i * 8),
        },
        trigger: float(raw, 0x650 + i * 4),
        held: Buttons(word(raw, 0x65C + i * 4)),
    };
    let analog = |i: usize| AnalogTimers {
        tilt: raw[0x670 + i],
        held: raw[0x673 + i],
        since_crossing: raw[0x676 + i],
        activity: raw[0x679 + i],
    };
    let mut input = FighterInput {
        current: frame(0),
        previous: frame(1),
        saved: frame(2),
        pressed: Buttons(word(raw, 0x668)),
        released: Buttons(word(raw, 0x66C)),
        horizontal: analog(0),
        vertical: analog(1),
        shoulder: analog(2),
        use_current_history: raw[0x221D] & 0x10 != 0,
        last_horizontal_positive: raw[0x2228] & 1 != 0,
        last_vertical_negative: raw[0x2229] & 0x80 != 0,
        ..FighterInput::default()
    };
    input.buttons.attack = raw[0x67C];
    input.buttons.special = raw[0x67D];
    input.buttons.jump_button = raw[0x67E];
    input.buttons.shield = raw[0x67F];
    input.buttons.digital_shield = raw[0x680];
    input.buttons.taunt = raw[0x681];
    input.buttons.down = raw[0x682];
    input.buttons.previous_attack = raw[0x683];
    input.buttons.previous_digital_shield = raw[0x684];
    input.buttons.jump = raw[0x685];
    input.buttons.special_up = raw[0x686];
    input.buttons.special_down = raw[0x687];
    input.buttons.special_side = raw[0x688];
    input.buttons.special_neutral = raw[0x689];
    input.buttons.previous_jump = raw[0x68A];
    input.buttons.previous_special_up = raw[0x68B];
    input
}
