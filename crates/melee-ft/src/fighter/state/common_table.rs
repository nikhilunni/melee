//! ftData_MotionStateList (ftmotionstates.c), indexed by ftCo_MS_* (0..341).
use super::{callbacks, unimplemented_row, ActionId, MotionRow, COMMON_COUNT};
use crate::fighter::CharacterCallbacks;
use melee_types::CommonMotionState as S;

pub const fn common_table<C: CharacterCallbacks>() -> [MotionRow<C>; COMMON_COUNT] {
    let mut rows = [unimplemented_row::<C>(); COMMON_COUNT];
    let mut index = 0;
    while index < COMMON_COUNT {
        rows[index].action = ActionId(index as u16);
        rows[index].id = S::ALL[index + 1]; // ALL begins with the -1 None sentinel.
        index += 1;
    }
    // ftCo_MS_DeadDown = 0; ftData_MotionStateList[0].
    rows[S::DeadDown as usize] = MotionRow {
        action: ActionId(0),
        id: S::DeadDown,
        animation: -1,
        anim: callbacks::animation::dead::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::dead::<C>,
        collision: callbacks::collision::thrown::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Rebirth = 12; ftData_MotionStateList[12].
    rows[S::Rebirth as usize] = MotionRow {
        action: ActionId(12),
        id: S::Rebirth,
        animation: 2,
        anim: callbacks::animation::revival::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::revival::<C>,
        collision: callbacks::collision::revival::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_RebirthWait = 13; ftData_MotionStateList[13].
    rows[S::RebirthWait as usize] = MotionRow {
        action: ActionId(13),
        id: S::RebirthWait,
        animation: 2,
        anim: callbacks::animation::revival::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::revival::<C>,
        collision: callbacks::collision::revival::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Wait = 14; ftData_MotionStateList[14].
    rows[S::Wait as usize] = MotionRow {
        action: ActionId(14),
        id: S::Wait,
        animation: 2,
        anim: callbacks::animation::wait::<C>,
        iasa: callbacks::input::wait::<C>,
        physics: callbacks::physics::wait::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_WalkSlow = 15; ftData_MotionStateList[15].
    rows[S::WalkSlow as usize] = MotionRow {
        action: ActionId(15),
        id: S::WalkSlow,
        animation: 7,
        anim: callbacks::animation::walk::<C>,
        iasa: callbacks::input::walk::<C>,
        physics: callbacks::physics::walk::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_WalkMiddle = 16; ftData_MotionStateList[16].
    rows[S::WalkMiddle as usize] = MotionRow {
        action: ActionId(16),
        id: S::WalkMiddle,
        animation: 8,
        anim: callbacks::animation::walk::<C>,
        iasa: callbacks::input::walk::<C>,
        physics: callbacks::physics::walk::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_WalkFast = 17; ftData_MotionStateList[17].
    rows[S::WalkFast as usize] = MotionRow {
        action: ActionId(17),
        id: S::WalkFast,
        animation: 9,
        anim: callbacks::animation::walk::<C>,
        iasa: callbacks::input::walk::<C>,
        physics: callbacks::physics::walk::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Turn = 18; ftData_MotionStateList[18].
    rows[S::Turn as usize] = MotionRow {
        action: ActionId(18),
        id: S::Turn,
        animation: 10,
        anim: callbacks::animation::turn::<C>,
        iasa: callbacks::input::turn::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_TurnRun = 19; ftData_MotionStateList[19].
    rows[S::TurnRun as usize] = MotionRow {
        action: ActionId(19),
        id: S::TurnRun,
        animation: 11,
        anim: callbacks::animation::turn_run::<C>,
        iasa: callbacks::input::turn_run::<C>,
        physics: callbacks::physics::turn_run::<C>,
        collision: callbacks::collision::turn_run::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Dash = 20; ftData_MotionStateList[20].
    rows[S::Dash as usize] = MotionRow {
        action: ActionId(20),
        id: S::Dash,
        animation: 12,
        anim: callbacks::animation::dash::<C>,
        iasa: callbacks::input::dash::<C>,
        physics: callbacks::physics::running::<C>,
        collision: callbacks::collision::running::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Run = 21; ftData_MotionStateList[21].
    rows[S::Run as usize] = MotionRow {
        action: ActionId(21),
        id: S::Run,
        animation: 13,
        anim: callbacks::animation::run::<C>,
        iasa: callbacks::input::run::<C>,
        physics: callbacks::physics::running::<C>,
        collision: callbacks::collision::running::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_RunBrake = 23; ftData_MotionStateList[23].
    rows[S::RunBrake as usize] = MotionRow {
        action: ActionId(23),
        id: S::RunBrake,
        animation: 14,
        anim: callbacks::animation::run_brake::<C>,
        iasa: callbacks::input::run_brake::<C>,
        physics: callbacks::physics::running::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_KneeBend = 24; ftData_MotionStateList[24].
    rows[S::KneeBend as usize] = MotionRow {
        action: ActionId(24),
        id: S::KneeBend,
        animation: 15,
        anim: callbacks::animation::knee_bend::<C>,
        iasa: callbacks::input::knee_bend::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_JumpF = 25; ftData_MotionStateList[25].
    rows[S::JumpF as usize] = MotionRow {
        action: ActionId(25),
        id: S::JumpF,
        animation: 16,
        anim: callbacks::animation::pass::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_JumpB = 26; ftData_MotionStateList[26].
    rows[S::JumpB as usize] = MotionRow {
        action: ActionId(26),
        id: S::JumpB,
        animation: 17,
        anim: callbacks::animation::pass::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_JumpAerialF = 27; ftData_MotionStateList[27].
    rows[S::JumpAerialF as usize] = MotionRow {
        action: ActionId(27),
        id: S::JumpAerialF,
        animation: 18,
        anim: callbacks::animation::pass::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_JumpAerialB = 28; ftData_MotionStateList[28].
    rows[S::JumpAerialB as usize] = MotionRow {
        action: ActionId(28),
        id: S::JumpAerialB,
        animation: 19,
        anim: callbacks::animation::pass::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Fall = 29; ftData_MotionStateList[29].
    rows[S::Fall as usize] = MotionRow {
        action: ActionId(29),
        id: S::Fall,
        animation: 20,
        anim: callbacks::animation::fall::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::fall::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_FallAerial = 32; ftData_MotionStateList[32].
    rows[S::FallAerial as usize] = MotionRow {
        action: ActionId(32),
        id: S::FallAerial,
        animation: 23,
        anim: callbacks::animation::fall::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::fall::<C>,
        collision: callbacks::collision::fall::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_FallSpecial = 35; ftData_MotionStateList[35].
    rows[S::FallSpecial as usize] = MotionRow {
        action: ActionId(35),
        id: S::FallSpecial,
        animation: 26,
        anim: callbacks::animation::fall::<C>,
        iasa: callbacks::input::fall_special::<C>,
        physics: callbacks::physics::fall_special::<C>,
        collision: callbacks::collision::fall_special::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DamageFall = 38; ftData_MotionStateList[38].
    rows[S::DamageFall as usize] = MotionRow {
        action: ActionId(38),
        id: S::DamageFall,
        animation: 29,
        anim: callbacks::animation::capture::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Squat = 39; ftData_MotionStateList[39].
    rows[S::Squat as usize] = MotionRow {
        action: ActionId(39),
        id: S::Squat,
        animation: 30,
        anim: callbacks::animation::squat::<C>,
        iasa: callbacks::input::squat::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_SquatWait = 40; ftData_MotionStateList[40].
    rows[S::SquatWait as usize] = MotionRow {
        action: ActionId(40),
        id: S::SquatWait,
        animation: 31,
        anim: callbacks::animation::squat_wait::<C>,
        iasa: callbacks::input::squat::<C>,
        physics: callbacks::physics::wait::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_SquatRv = 41; ftData_MotionStateList[41].
    rows[S::SquatRv as usize] = MotionRow {
        action: ActionId(41),
        id: S::SquatRv,
        animation: 34,
        anim: callbacks::animation::squat::<C>,
        iasa: callbacks::input::squat::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Landing = 42; ftData_MotionStateList[42].
    rows[S::Landing as usize] = MotionRow {
        action: ActionId(42),
        id: S::Landing,
        animation: 35,
        anim: callbacks::animation::landing::<C>,
        iasa: callbacks::input::landing::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_LandingFallSpecial = 43; ftData_MotionStateList[43].
    rows[S::LandingFallSpecial as usize] = MotionRow {
        action: ActionId(43),
        id: S::LandingFallSpecial,
        animation: 36,
        anim: callbacks::animation::landing::<C>,
        iasa: callbacks::input::landing::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_wait::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Attack11 = 44; ftData_MotionStateList[44].
    rows[S::Attack11 as usize] = MotionRow {
        action: ActionId(44),
        id: S::Attack11,
        animation: 46,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::jab::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[45].
    rows[S::Attack12 as usize] = MotionRow {
        action: ActionId(45),
        id: S::Attack12,
        animation: 47,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::jab::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[46].
    rows[S::Attack13 as usize] = MotionRow {
        action: ActionId(46),
        id: S::Attack13,
        animation: 48,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[47].
    rows[S::Attack100Start as usize] = MotionRow {
        action: ActionId(47),
        id: S::Attack100Start,
        animation: 49,
        anim: callbacks::animation::rapid_start::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[48].
    rows[S::Attack100Loop as usize] = MotionRow {
        action: ActionId(48),
        id: S::Attack100Loop,
        animation: 50,
        anim: callbacks::animation::rapid_loop::<C>,
        iasa: callbacks::input::rapid_loop::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[49].
    rows[S::Attack100End as usize] = MotionRow {
        action: ActionId(49),
        id: S::Attack100End,
        animation: 51,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_AttackHi3 = 56; ftData_MotionStateList[56].
    rows[S::AttackHi3 as usize] = MotionRow {
        action: ActionId(56),
        id: S::AttackHi3,
        animation: 58,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_AttackS4S = 60; ftData_MotionStateList[60].
    rows[S::AttackS4S as usize] = MotionRow {
        action: ActionId(60),
        id: S::AttackS4S,
        animation: 62,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DamageHi3 = 77; ftData_MotionStateList[77].
    rows[S::DamageHi3 as usize] = MotionRow {
        action: ActionId(77),
        id: S::DamageHi3,
        animation: 167,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DamageN1 = 78; ftData_MotionStateList[78].
    rows[S::DamageN1 as usize] = MotionRow {
        action: ActionId(78),
        id: S::DamageN1,
        animation: 168,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DamageN2 = 79; ftData_MotionStateList[79].
    rows[S::DamageN2 as usize] = MotionRow {
        action: ActionId(79),
        id: S::DamageN2,
        animation: 169,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DamageFlyN = 88; ftData_MotionStateList[88].
    rows[S::DamageFlyN as usize] = MotionRow {
        action: ActionId(88),
        id: S::DamageFlyN,
        animation: 178,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[51], retail ftmotionstates.c.
    rows[S::AttackS3Hi as usize] = MotionRow {
        action: ActionId(51),
        id: S::AttackS3Hi,
        animation: 53,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[52], retail ftmotionstates.c.
    rows[S::AttackS3HiS as usize] = MotionRow {
        action: ActionId(52),
        id: S::AttackS3HiS,
        animation: 54,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[53], retail ftmotionstates.c.
    rows[S::AttackS3S as usize] = MotionRow {
        action: ActionId(53),
        id: S::AttackS3S,
        animation: 55,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[54], retail ftmotionstates.c.
    rows[S::AttackS3LwS as usize] = MotionRow {
        action: ActionId(54),
        id: S::AttackS3LwS,
        animation: 56,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[55], retail ftmotionstates.c.
    rows[S::AttackS3Lw as usize] = MotionRow {
        action: ActionId(55),
        id: S::AttackS3Lw,
        animation: 57,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[63], retail ftmotionstates.c.
    rows[S::AttackHi4 as usize] = MotionRow {
        action: ActionId(63),
        id: S::AttackHi4,
        animation: 66,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[64], retail ftmotionstates.c.
    rows[S::AttackLw4 as usize] = MotionRow {
        action: ActionId(64),
        id: S::AttackLw4,
        animation: 67,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[50], retail ftmotionstates.c.
    rows[S::AttackDash as usize] = MotionRow {
        action: ActionId(50),
        id: S::AttackDash,
        animation: 52,
        anim: callbacks::animation::jab::<C>,
        iasa: callbacks::input::tilt::<C>,
        physics: callbacks::physics::dash_attack::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[57], retail ftmotionstates.c.
    rows[S::AttackLw3 as usize] = MotionRow {
        action: ActionId(57),
        id: S::AttackLw3,
        animation: 59,
        anim: callbacks::animation::down_tilt::<C>,
        iasa: callbacks::input::down_tilt::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[75], retail ftmotionstates.c.
    rows[S::DamageHi1 as usize] = MotionRow {
        action: ActionId(75),
        id: S::DamageHi1,
        animation: 165,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[76], retail ftmotionstates.c.
    rows[S::DamageHi2 as usize] = MotionRow {
        action: ActionId(76),
        id: S::DamageHi2,
        animation: 166,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[80], retail ftmotionstates.c.
    rows[S::DamageN3 as usize] = MotionRow {
        action: ActionId(80),
        id: S::DamageN3,
        animation: 170,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[81], retail ftmotionstates.c.
    rows[S::DamageLw1 as usize] = MotionRow {
        action: ActionId(81),
        id: S::DamageLw1,
        animation: 171,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[82], retail ftmotionstates.c.
    rows[S::DamageLw2 as usize] = MotionRow {
        action: ActionId(82),
        id: S::DamageLw2,
        animation: 172,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[83], retail ftmotionstates.c.
    rows[S::DamageLw3 as usize] = MotionRow {
        action: ActionId(83),
        id: S::DamageLw3,
        animation: 173,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[84], retail ftmotionstates.c.
    rows[S::DamageAir1 as usize] = MotionRow {
        action: ActionId(84),
        id: S::DamageAir1,
        animation: 174,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[85], retail ftmotionstates.c.
    rows[S::DamageAir2 as usize] = MotionRow {
        action: ActionId(85),
        id: S::DamageAir2,
        animation: 175,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[86], retail ftmotionstates.c.
    rows[S::DamageAir3 as usize] = MotionRow {
        action: ActionId(86),
        id: S::DamageAir3,
        animation: 176,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[87], retail ftmotionstates.c.
    rows[S::DamageFlyHi as usize] = MotionRow {
        action: ActionId(87),
        id: S::DamageFlyHi,
        animation: 177,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[89], retail ftmotionstates.c.
    rows[S::DamageFlyLw as usize] = MotionRow {
        action: ActionId(89),
        id: S::DamageFlyLw,
        animation: 179,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[90], retail ftmotionstates.c.
    rows[S::DamageFlyTop as usize] = MotionRow {
        action: ActionId(90),
        id: S::DamageFlyTop,
        animation: 180,
        anim: callbacks::animation::damage::<C>,
        iasa: callbacks::input::damage::<C>,
        physics: callbacks::physics::damage::<C>,
        collision: callbacks::collision::damage::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[183], retail ftmotionstates.c.
    rows[S::DownBoundU as usize] = MotionRow {
        action: ActionId(183),
        id: S::DownBoundU,
        animation: 183,
        anim: callbacks::animation::down_bound::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::down::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftData_MotionStateList[184], retail ftmotionstates.c.
    rows[S::DownWaitU as usize] = MotionRow {
        action: ActionId(184),
        id: S::DownWaitU,
        animation: 184,
        anim: callbacks::animation::down_bound::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::down::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_GuardOn = 178; ftData_MotionStateList[178].
    rows[S::GuardOn as usize] = MotionRow {
        action: ActionId(178),
        id: S::GuardOn,
        animation: 37,
        anim: callbacks::animation::guard_on::<C>,
        iasa: callbacks::input::guard_on::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Guard = 179; ftData_MotionStateList[179].
    rows[S::Guard as usize] = MotionRow {
        action: ActionId(179),
        id: S::Guard,
        animation: 38,
        anim: callbacks::animation::guard_on::<C>,
        iasa: callbacks::input::guard_on::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_GuardOff = 180; ftData_MotionStateList[180].
    rows[S::GuardOff as usize] = MotionRow {
        action: ActionId(180),
        id: S::GuardOff,
        animation: 39,
        anim: callbacks::animation::guard_on::<C>,
        iasa: callbacks::input::guard_on::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_GuardSetOff = 181; ftData_MotionStateList[181].
    rows[S::GuardSetOff as usize] = MotionRow {
        action: ActionId(181),
        id: S::GuardSetOff,
        animation: 40,
        anim: callbacks::animation::guard_on::<C>,
        iasa: callbacks::input::guard_on::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::guard_set_off::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_GuardReflect = 182; ftData_MotionStateList[182].
    rows[S::GuardReflect as usize] = MotionRow {
        action: ActionId(182),
        id: S::GuardReflect,
        animation: 37,
        anim: callbacks::animation::guard_on::<C>,
        iasa: callbacks::input::guard_on::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::ground_action::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DownBoundD = 191; ftData_MotionStateList[191].
    rows[S::DownBoundD as usize] = MotionRow {
        action: ActionId(191),
        id: S::DownBoundD,
        animation: 191,
        anim: callbacks::animation::down_bound::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::down::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_DownWaitD = 192; ftData_MotionStateList[192].
    rows[S::DownWaitD as usize] = MotionRow {
        action: ActionId(192),
        id: S::DownWaitD,
        animation: 192,
        anim: callbacks::animation::down_bound::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::down::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_PassiveStandB = 201; ftData_MotionStateList[201].
    rows[S::PassiveStandB as usize] = MotionRow {
        action: ActionId(201),
        id: S::PassiveStandB,
        animation: 201,
        anim: callbacks::animation::tech_roll::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Catch = 212; ftData_MotionStateList[212].
    rows[S::Catch as usize] = MotionRow {
        action: ActionId(212),
        id: S::Catch,
        animation: 242,
        anim: callbacks::animation::catch::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::catch::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CatchPull = 213; ftData_MotionStateList[213].
    rows[S::CatchPull as usize] = MotionRow {
        action: ActionId(213),
        id: S::CatchPull,
        animation: 242,
        anim: callbacks::animation::catch_pull::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::catch::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CatchWait = 216; ftData_MotionStateList[216].
    rows[S::CatchWait as usize] = MotionRow {
        action: ActionId(216),
        id: S::CatchWait,
        animation: 244,
        anim: callbacks::animation::capture::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::catch::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_ThrowB = 220; ftData_MotionStateList[220].
    rows[S::ThrowB as usize] = MotionRow {
        action: ActionId(220),
        id: S::ThrowB,
        animation: 248,
        anim: callbacks::animation::throw::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::jab::<C>,
        collision: callbacks::collision::catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CapturePulledLw = 226; ftData_MotionStateList[226].
    rows[S::CapturePulledLw as usize] = MotionRow {
        action: ActionId(226),
        id: S::CapturePulledLw,
        animation: 254,
        anim: callbacks::animation::capture::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::capture::<C>,
        collision: callbacks::collision::capture::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CaptureWaitLw = 227; ftData_MotionStateList[227].
    rows[S::CaptureWaitLw as usize] = MotionRow {
        action: ActionId(227),
        id: S::CaptureWaitLw,
        animation: 255,
        anim: callbacks::animation::capture::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::capture::<C>,
        collision: callbacks::collision::capture::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EscapeF = 233; ftData_MotionStateList[233].
    rows[S::EscapeF as usize] = MotionRow {
        action: ActionId(233),
        id: S::EscapeF,
        animation: 42,
        anim: callbacks::animation::escape::<C>,
        iasa: callbacks::input::escape::<C>,
        physics: callbacks::physics::escape::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EscapeB = 234; ftData_MotionStateList[234].
    rows[S::EscapeB as usize] = MotionRow {
        action: ActionId(234),
        id: S::EscapeB,
        animation: 43,
        anim: callbacks::animation::escape::<C>,
        iasa: callbacks::input::escape::<C>,
        physics: callbacks::physics::escape::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EscapeN = 235; ftData_MotionStateList[235].
    rows[S::EscapeN as usize] = MotionRow {
        action: ActionId(235),
        id: S::EscapeN,
        animation: 41,
        anim: callbacks::animation::escape::<C>,
        iasa: callbacks::input::escape_n::<C>,
        physics: callbacks::physics::guard_on::<C>,
        collision: callbacks::collision::escape::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EscapeAir = 236; ftData_MotionStateList[236].
    rows[S::EscapeAir as usize] = MotionRow {
        action: ActionId(236),
        id: S::EscapeAir,
        animation: 44,
        anim: callbacks::animation::escape_air::<C>,
        iasa: callbacks::input::escape_air::<C>,
        physics: callbacks::physics::escape_air::<C>,
        collision: callbacks::collision::escape_air::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_ThrownB = 240; ftData_MotionStateList[240].
    rows[S::ThrownB as usize] = MotionRow {
        action: ActionId(240),
        id: S::ThrownB,
        animation: 263,
        anim: callbacks::animation::thrown::<C>,
        iasa: callbacks::input::catch::<C>,
        physics: callbacks::physics::capture::<C>,
        collision: callbacks::collision::thrown::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Pass = 244; ftData_MotionStateList[244].
    rows[S::Pass as usize] = MotionRow {
        action: ActionId(244),
        id: S::Pass,
        animation: 209,
        anim: callbacks::animation::pass::<C>,
        iasa: callbacks::input::aerial::<C>,
        physics: callbacks::physics::pass::<C>,
        collision: callbacks::collision::pass::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffCatch = 252; ftData_MotionStateList[252].
    rows[S::CliffCatch as usize] = MotionRow {
        action: ActionId(252),
        id: S::CliffCatch,
        animation: 216,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_catch::<C>,
        physics: callbacks::physics::cliff_catch::<C>,
        collision: callbacks::collision::cliff_catch::<C>,
        camera: callbacks::camera::cliff::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffWait = 253; ftData_MotionStateList[253].
    rows[S::CliffWait as usize] = MotionRow {
        action: ActionId(253),
        id: S::CliffWait,
        animation: 217,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_wait::<C>,
        physics: callbacks::physics::cliff_catch::<C>,
        collision: callbacks::collision::cliff_catch::<C>,
        camera: callbacks::camera::cliff::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffClimbQuick = 255; ftData_MotionStateList[255].
    rows[S::CliffClimbQuick as usize] = MotionRow {
        action: ActionId(255),
        id: S::CliffClimbQuick,
        animation: 220,
        anim: callbacks::animation::cliff_climb::<C>,
        iasa: callbacks::input::cliff_climb::<C>,
        physics: callbacks::physics::cliff_climb::<C>,
        collision: callbacks::collision::cliff_climb::<C>,
        camera: callbacks::camera::cliff::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffEscapeQuick = 259; ftData_MotionStateList[259].
    rows[S::CliffEscapeQuick as usize] = MotionRow {
        action: ActionId(259),
        id: S::CliffEscapeQuick,
        animation: 224,
        anim: callbacks::animation::cliff_climb::<C>,
        iasa: callbacks::input::cliff_climb::<C>,
        physics: callbacks::physics::cliff_climb::<C>,
        collision: callbacks::collision::cliff_climb::<C>,
        camera: callbacks::camera::cliff::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffJumpSlow1 = 260; ftData_MotionStateList[260].
    rows[S::CliffJumpSlow1 as usize] = MotionRow {
        action: ActionId(260),
        id: S::CliffJumpSlow1,
        animation: 225,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_catch::<C>,
        physics: callbacks::physics::cliff_catch::<C>,
        collision: callbacks::collision::cliff_catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffJumpSlow2 = 261; ftData_MotionStateList[261].
    rows[S::CliffJumpSlow2 as usize] = MotionRow {
        action: ActionId(261),
        id: S::CliffJumpSlow2,
        animation: 226,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_catch::<C>,
        physics: callbacks::physics::cliff_jump2::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffJumpQuick1 = 262; ftData_MotionStateList[262].
    rows[S::CliffJumpQuick1 as usize] = MotionRow {
        action: ActionId(262),
        id: S::CliffJumpQuick1,
        animation: 227,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_catch::<C>,
        physics: callbacks::physics::cliff_catch::<C>,
        collision: callbacks::collision::cliff_catch::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_CliffJumpQuick2 = 263; ftData_MotionStateList[263].
    rows[S::CliffJumpQuick2 as usize] = MotionRow {
        action: ActionId(263),
        id: S::CliffJumpQuick2,
        animation: 228,
        anim: callbacks::animation::cliff_catch::<C>,
        iasa: callbacks::input::cliff_catch::<C>,
        physics: callbacks::physics::cliff_jump2::<C>,
        collision: callbacks::collision::jump::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_Entry = 322; ftData_MotionStateList[322].
    rows[S::Entry as usize] = MotionRow {
        action: ActionId(322),
        id: S::Entry,
        animation: -1,
        anim: callbacks::animation::entry::<C>,
        iasa: callbacks::input::entry::<C>,
        physics: callbacks::physics::entry::<C>,
        collision: callbacks::collision::entry::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EntryStart = 323; ftData_MotionStateList[323].
    rows[S::EntryStart as usize] = MotionRow {
        action: ActionId(323),
        id: S::EntryStart,
        animation: 238,
        anim: callbacks::animation::entry::<C>,
        iasa: callbacks::input::entry::<C>,
        physics: callbacks::physics::entry::<C>,
        collision: callbacks::collision::entry::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    // ftCo_MS_EntryEnd = 324; ftData_MotionStateList[324].
    rows[S::EntryEnd as usize] = MotionRow {
        action: ActionId(324),
        id: S::EntryEnd,
        animation: -1,
        anim: callbacks::animation::entry::<C>,
        iasa: callbacks::input::entry::<C>,
        physics: callbacks::physics::entry::<C>,
        collision: callbacks::collision::entry::<C>,
        camera: callbacks::camera::follow_fighter::<C>,
        implemented: true,
    };
    rows
}
