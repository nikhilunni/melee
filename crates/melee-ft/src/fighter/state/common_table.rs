//! ftData_MotionStateList (ftmotionstates.c), indexed by ftCo_MS_* (0..341).
use super::{callbacks, unimplemented_row, ActionId, MotionRow, COMMON_COUNT};
use melee_types::CommonMotionState as S;

pub static COMMON: [MotionRow; COMMON_COUNT] = common_table();

pub const fn common_table() -> [MotionRow; COMMON_COUNT] {
    let mut rows = [unimplemented_row(); COMMON_COUNT];
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
        anim: callbacks::animation::dead,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::dead,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DeadLeft = 1 / ftCo_MS_DeadRight = 2; ftData_MotionStateList[1..=2].
    rows[S::DeadLeft as usize] = MotionRow {
        action: ActionId(1),
        id: S::DeadLeft,
        animation: -1,
        anim: callbacks::animation::dead,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::dead,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::DeadRight as usize] = MotionRow {
        action: ActionId(2),
        id: S::DeadRight,
        animation: -1,
        anim: callbacks::animation::dead,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::dead,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DeadUpStar = 4; ftData_MotionStateList[4]: ftCo_SM_DamageFall (29).
    rows[S::DeadUpStar as usize] = MotionRow {
        action: ActionId(4),
        id: S::DeadUpStar,
        animation: 29,
        anim: callbacks::animation::dead_star,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::dead_star,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DeadUpFall = 6; ftData_MotionStateList[6]: ftCo_SM_DamageFall (29).
    rows[S::DeadUpFall as usize] = MotionRow {
        action: ActionId(6),
        id: S::DeadUpFall,
        animation: 29,
        anim: callbacks::animation::dead_screen,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::dead,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Rebirth = 12; ftData_MotionStateList[12].
    rows[S::Rebirth as usize] = MotionRow {
        action: ActionId(12),
        id: S::Rebirth,
        animation: 2,
        anim: callbacks::animation::revival,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::revival,
        collision: callbacks::collision::revival,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_RebirthWait = 13; ftData_MotionStateList[13].
    rows[S::RebirthWait as usize] = MotionRow {
        action: ActionId(13),
        id: S::RebirthWait,
        animation: 2,
        anim: callbacks::animation::revival,
        iasa: callbacks::input::revival,
        physics: callbacks::physics::revival,
        collision: callbacks::collision::revival,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Wait = 14; ftData_MotionStateList[14].
    rows[S::Wait as usize] = MotionRow {
        action: ActionId(14),
        id: S::Wait,
        animation: 2,
        anim: callbacks::animation::wait,
        iasa: callbacks::input::wait,
        physics: callbacks::physics::wait,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_WalkSlow = 15; ftData_MotionStateList[15].
    rows[S::WalkSlow as usize] = MotionRow {
        action: ActionId(15),
        id: S::WalkSlow,
        animation: 7,
        anim: callbacks::animation::walk,
        iasa: callbacks::input::walk,
        physics: callbacks::physics::walk,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_WalkMiddle = 16; ftData_MotionStateList[16].
    rows[S::WalkMiddle as usize] = MotionRow {
        action: ActionId(16),
        id: S::WalkMiddle,
        animation: 8,
        anim: callbacks::animation::walk,
        iasa: callbacks::input::walk,
        physics: callbacks::physics::walk,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_WalkFast = 17; ftData_MotionStateList[17].
    rows[S::WalkFast as usize] = MotionRow {
        action: ActionId(17),
        id: S::WalkFast,
        animation: 9,
        anim: callbacks::animation::walk,
        iasa: callbacks::input::walk,
        physics: callbacks::physics::walk,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Turn = 18; ftData_MotionStateList[18].
    rows[S::Turn as usize] = MotionRow {
        action: ActionId(18),
        id: S::Turn,
        animation: 10,
        anim: callbacks::animation::turn,
        iasa: callbacks::input::turn,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_TurnRun = 19; ftData_MotionStateList[19].
    rows[S::TurnRun as usize] = MotionRow {
        action: ActionId(19),
        id: S::TurnRun,
        animation: 11,
        anim: callbacks::animation::turn_run,
        iasa: callbacks::input::turn_run,
        physics: callbacks::physics::turn_run,
        collision: callbacks::collision::turn_run,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Dash = 20; ftData_MotionStateList[20].
    rows[S::Dash as usize] = MotionRow {
        action: ActionId(20),
        id: S::Dash,
        animation: 12,
        anim: callbacks::animation::dash,
        iasa: callbacks::input::dash,
        physics: callbacks::physics::running,
        collision: callbacks::collision::running,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Run = 21; ftData_MotionStateList[21].
    rows[S::Run as usize] = MotionRow {
        action: ActionId(21),
        id: S::Run,
        animation: 13,
        anim: callbacks::animation::run,
        iasa: callbacks::input::run,
        physics: callbacks::physics::running,
        collision: callbacks::collision::running,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_RunBrake = 23; ftData_MotionStateList[23].
    rows[S::RunBrake as usize] = MotionRow {
        action: ActionId(23),
        id: S::RunBrake,
        animation: 14,
        anim: callbacks::animation::run_brake,
        iasa: callbacks::input::run_brake,
        physics: callbacks::physics::running,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_KneeBend = 24; ftData_MotionStateList[24].
    rows[S::KneeBend as usize] = MotionRow {
        action: ActionId(24),
        id: S::KneeBend,
        animation: 15,
        anim: callbacks::animation::knee_bend,
        iasa: callbacks::input::knee_bend,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_JumpF = 25; ftData_MotionStateList[25].
    rows[S::JumpF as usize] = MotionRow {
        action: ActionId(25),
        id: S::JumpF,
        animation: 16,
        anim: callbacks::animation::pass,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_JumpB = 26; ftData_MotionStateList[26].
    rows[S::JumpB as usize] = MotionRow {
        action: ActionId(26),
        id: S::JumpB,
        animation: 17,
        anim: callbacks::animation::pass,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_JumpAerialF = 27; ftData_MotionStateList[27].
    rows[S::JumpAerialF as usize] = MotionRow {
        action: ActionId(27),
        id: S::JumpAerialF,
        animation: 18,
        anim: callbacks::animation::pass,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_JumpAerialB = 28; ftData_MotionStateList[28].
    rows[S::JumpAerialB as usize] = MotionRow {
        action: ActionId(28),
        id: S::JumpAerialB,
        animation: 19,
        anim: callbacks::animation::pass,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Fall = 29; ftData_MotionStateList[29].
    rows[S::Fall as usize] = MotionRow {
        action: ActionId(29),
        id: S::Fall,
        animation: 20,
        anim: callbacks::animation::fall,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::fall,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_FallAerial = 32; ftData_MotionStateList[32].
    rows[S::FallAerial as usize] = MotionRow {
        action: ActionId(32),
        id: S::FallAerial,
        animation: 23,
        anim: callbacks::animation::fall,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::fall,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_FallSpecial = 35; ftData_MotionStateList[35].
    rows[S::FallSpecial as usize] = MotionRow {
        action: ActionId(35),
        id: S::FallSpecial,
        animation: 26,
        anim: callbacks::animation::fall,
        iasa: callbacks::input::fall_special,
        physics: callbacks::physics::fall_special,
        collision: callbacks::collision::fall_special,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MissFoot: backward ledge slip, common submotion 215.
    rows[S::MissFoot as usize] = MotionRow {
        action: ActionId(251),
        id: S::MissFoot,
        animation: 215,
        anim: super::super::teeter::missed_footing_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::fall,
        collision: callbacks::collision::pass,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DamageFall = 38; ftData_MotionStateList[38].
    rows[S::DamageFall as usize] = MotionRow {
        action: ActionId(38),
        id: S::DamageFall,
        animation: 29,
        anim: callbacks::animation::capture,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Squat = 39; ftData_MotionStateList[39].
    rows[S::Squat as usize] = MotionRow {
        action: ActionId(39),
        id: S::Squat,
        animation: 30,
        anim: callbacks::animation::squat,
        iasa: callbacks::input::squat,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_SquatWait = 40; ftData_MotionStateList[40].
    rows[S::SquatWait as usize] = MotionRow {
        action: ActionId(40),
        id: S::SquatWait,
        animation: 31,
        anim: callbacks::animation::squat_wait,
        iasa: callbacks::input::squat,
        physics: callbacks::physics::wait,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_SquatRv = 41; ftData_MotionStateList[41].
    rows[S::SquatRv as usize] = MotionRow {
        action: ActionId(41),
        id: S::SquatRv,
        animation: 34,
        anim: callbacks::animation::squat,
        iasa: callbacks::input::squat,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Landing = 42; ftData_MotionStateList[42].
    rows[S::Landing as usize] = MotionRow {
        action: ActionId(42),
        id: S::Landing,
        animation: 35,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::landing,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_LandingFallSpecial = 43; ftData_MotionStateList[43].
    rows[S::LandingFallSpecial as usize] = MotionRow {
        action: ActionId(43),
        id: S::LandingFallSpecial,
        animation: 36,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::landing,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Attack11 = 44; ftData_MotionStateList[44].
    rows[S::Attack11 as usize] = MotionRow {
        action: ActionId(44),
        id: S::Attack11,
        animation: 46,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::jab,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[45].
    rows[S::Attack12 as usize] = MotionRow {
        action: ActionId(45),
        id: S::Attack12,
        animation: 47,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::jab,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[46].
    rows[S::Attack13 as usize] = MotionRow {
        action: ActionId(46),
        id: S::Attack13,
        animation: 48,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[47].
    rows[S::Attack100Start as usize] = MotionRow {
        action: ActionId(47),
        id: S::Attack100Start,
        animation: 49,
        anim: callbacks::animation::rapid_start,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[48].
    rows[S::Attack100Loop as usize] = MotionRow {
        action: ActionId(48),
        id: S::Attack100Loop,
        animation: 50,
        anim: callbacks::animation::rapid_loop,
        iasa: callbacks::input::rapid_loop,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[49].
    rows[S::Attack100End as usize] = MotionRow {
        action: ActionId(49),
        id: S::Attack100End,
        animation: 51,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_AttackHi3 = 56; ftData_MotionStateList[56].
    rows[S::AttackHi3 as usize] = MotionRow {
        action: ActionId(56),
        id: S::AttackHi3,
        animation: 58,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_AttackS4S = 60; ftData_MotionStateList[60].
    rows[S::AttackS4S as usize] = MotionRow {
        action: ActionId(60),
        id: S::AttackS4S,
        animation: 62,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DamageHi3 = 77; ftData_MotionStateList[77].
    rows[S::DamageHi3 as usize] = MotionRow {
        action: ActionId(77),
        id: S::DamageHi3,
        animation: 167,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DamageN1 = 78; ftData_MotionStateList[78].
    rows[S::DamageN1 as usize] = MotionRow {
        action: ActionId(78),
        id: S::DamageN1,
        animation: 168,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DamageN2 = 79; ftData_MotionStateList[79].
    rows[S::DamageN2 as usize] = MotionRow {
        action: ActionId(79),
        id: S::DamageN2,
        animation: 169,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DamageFlyN = 88; ftData_MotionStateList[88].
    rows[S::DamageFlyN as usize] = MotionRow {
        action: ActionId(88),
        id: S::DamageFlyN,
        animation: 178,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[51], retail ftmotionstates.c.
    rows[S::AttackS3Hi as usize] = MotionRow {
        action: ActionId(51),
        id: S::AttackS3Hi,
        animation: 53,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[52], retail ftmotionstates.c.
    rows[S::AttackS3HiS as usize] = MotionRow {
        action: ActionId(52),
        id: S::AttackS3HiS,
        animation: 54,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[53], retail ftmotionstates.c.
    rows[S::AttackS3S as usize] = MotionRow {
        action: ActionId(53),
        id: S::AttackS3S,
        animation: 55,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[54], retail ftmotionstates.c.
    rows[S::AttackS3LwS as usize] = MotionRow {
        action: ActionId(54),
        id: S::AttackS3LwS,
        animation: 56,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[55], retail ftmotionstates.c.
    rows[S::AttackS3Lw as usize] = MotionRow {
        action: ActionId(55),
        id: S::AttackS3Lw,
        animation: 57,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[63], retail ftmotionstates.c.
    rows[S::AttackHi4 as usize] = MotionRow {
        action: ActionId(63),
        id: S::AttackHi4,
        animation: 66,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[64], retail ftmotionstates.c.
    rows[S::AttackLw4 as usize] = MotionRow {
        action: ActionId(64),
        id: S::AttackLw4,
        animation: 67,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[50], retail ftmotionstates.c.
    rows[S::AttackDash as usize] = MotionRow {
        action: ActionId(50),
        id: S::AttackDash,
        animation: 52,
        anim: callbacks::animation::jab,
        iasa: callbacks::input::tilt,
        physics: callbacks::physics::dash_attack,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[57], retail ftmotionstates.c.
    rows[S::AttackLw3 as usize] = MotionRow {
        action: ActionId(57),
        id: S::AttackLw3,
        animation: 59,
        anim: callbacks::animation::down_tilt,
        iasa: callbacks::input::down_tilt,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[75], retail ftmotionstates.c.
    rows[S::DamageHi1 as usize] = MotionRow {
        action: ActionId(75),
        id: S::DamageHi1,
        animation: 165,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[76], retail ftmotionstates.c.
    rows[S::DamageHi2 as usize] = MotionRow {
        action: ActionId(76),
        id: S::DamageHi2,
        animation: 166,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[80], retail ftmotionstates.c.
    rows[S::DamageN3 as usize] = MotionRow {
        action: ActionId(80),
        id: S::DamageN3,
        animation: 170,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[81], retail ftmotionstates.c.
    rows[S::DamageLw1 as usize] = MotionRow {
        action: ActionId(81),
        id: S::DamageLw1,
        animation: 171,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[82], retail ftmotionstates.c.
    rows[S::DamageLw2 as usize] = MotionRow {
        action: ActionId(82),
        id: S::DamageLw2,
        animation: 172,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[83], retail ftmotionstates.c.
    rows[S::DamageLw3 as usize] = MotionRow {
        action: ActionId(83),
        id: S::DamageLw3,
        animation: 173,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[84], retail ftmotionstates.c.
    rows[S::DamageAir1 as usize] = MotionRow {
        action: ActionId(84),
        id: S::DamageAir1,
        animation: 174,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[85], retail ftmotionstates.c.
    rows[S::DamageAir2 as usize] = MotionRow {
        action: ActionId(85),
        id: S::DamageAir2,
        animation: 175,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[86], retail ftmotionstates.c.
    rows[S::DamageAir3 as usize] = MotionRow {
        action: ActionId(86),
        id: S::DamageAir3,
        animation: 176,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[87], retail ftmotionstates.c.
    rows[S::DamageFlyHi as usize] = MotionRow {
        action: ActionId(87),
        id: S::DamageFlyHi,
        animation: 177,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[89], retail ftmotionstates.c.
    rows[S::DamageFlyLw as usize] = MotionRow {
        action: ActionId(89),
        id: S::DamageFlyLw,
        animation: 179,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[90], retail ftmotionstates.c.
    rows[S::DamageFlyTop as usize] = MotionRow {
        action: ActionId(90),
        id: S::DamageFlyTop,
        animation: 180,
        anim: callbacks::animation::damage,
        iasa: callbacks::input::damage,
        physics: callbacks::physics::damage,
        collision: callbacks::collision::damage,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[183], retail ftmotionstates.c.
    rows[S::DownBoundU as usize] = MotionRow {
        action: ActionId(183),
        id: S::DownBoundU,
        animation: 183,
        anim: callbacks::animation::down_bound,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[184], retail ftmotionstates.c.
    rows[S::DownWaitU as usize] = MotionRow {
        action: ActionId(184),
        id: S::DownWaitU,
        animation: 184,
        anim: callbacks::animation::down_bound,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_GuardOn = 178; ftData_MotionStateList[178].
    rows[S::GuardOn as usize] = MotionRow {
        action: ActionId(178),
        id: S::GuardOn,
        animation: 37,
        anim: callbacks::animation::guard_on,
        iasa: callbacks::input::guard_on,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Guard = 179; ftData_MotionStateList[179].
    rows[S::Guard as usize] = MotionRow {
        action: ActionId(179),
        id: S::Guard,
        animation: 38,
        anim: callbacks::animation::guard_on,
        iasa: callbacks::input::guard_on,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_GuardOff = 180; ftData_MotionStateList[180].
    rows[S::GuardOff as usize] = MotionRow {
        action: ActionId(180),
        id: S::GuardOff,
        animation: 39,
        anim: callbacks::animation::guard_on,
        iasa: callbacks::input::guard_on,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_GuardSetOff = 181; ftData_MotionStateList[181].
    rows[S::GuardSetOff as usize] = MotionRow {
        action: ActionId(181),
        id: S::GuardSetOff,
        animation: 40,
        anim: callbacks::animation::guard_on,
        iasa: callbacks::input::guard_on,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::guard_set_off,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_GuardReflect = 182; ftData_MotionStateList[182].
    rows[S::GuardReflect as usize] = MotionRow {
        action: ActionId(182),
        id: S::GuardReflect,
        animation: 37,
        anim: callbacks::animation::guard_on,
        iasa: callbacks::input::guard_on,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DownBoundD = 191; ftData_MotionStateList[191].
    rows[S::DownBoundD as usize] = MotionRow {
        action: ActionId(191),
        id: S::DownBoundD,
        animation: 191,
        anim: callbacks::animation::down_bound,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_DownWaitD = 192; ftData_MotionStateList[192].
    rows[S::DownWaitD as usize] = MotionRow {
        action: ActionId(192),
        id: S::DownWaitD,
        animation: 192,
        anim: callbacks::animation::down_bound,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_Passive.c / ftCo_PassiveStand.c: neutral and forward tech.
    rows[S::Passive as usize] = MotionRow {
        action: ActionId(199),
        id: S::Passive,
        animation: 199,
        anim: callbacks::animation::tech_roll,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::PassiveStandF as usize] = MotionRow {
        action: ActionId(200),
        id: S::PassiveStandF,
        animation: 200,
        anim: callbacks::animation::tech_roll,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_PassiveStandB = 201; ftData_MotionStateList[201].
    rows[S::PassiveStandB as usize] = MotionRow {
        action: ActionId(201),
        id: S::PassiveStandB,
        animation: 201,
        anim: callbacks::animation::tech_roll,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Catch = 212; ftData_MotionStateList[212].
    rows[S::Catch as usize] = MotionRow {
        action: ActionId(212),
        id: S::Catch,
        animation: 242,
        anim: callbacks::animation::catch,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::catch,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CatchPull = 213; ftData_MotionStateList[213].
    rows[S::CatchPull as usize] = MotionRow {
        action: ActionId(213),
        id: S::CatchPull,
        animation: 242,
        anim: callbacks::animation::catch_pull,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::catch,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::CatchDash as usize] = MotionRow {
        action: ActionId(214),
        id: S::CatchDash,
        animation: 243,
        physics: callbacks::physics::catch_dash,
        ..rows[S::Catch as usize]
    };
    rows[S::CatchDashPull as usize] = MotionRow {
        action: ActionId(215),
        id: S::CatchDashPull,
        animation: 243,
        ..rows[S::CatchPull as usize]
    };
    // ftCo_MS_CatchWait = 216; ftData_MotionStateList[216].
    rows[S::CatchWait as usize] = MotionRow {
        action: ActionId(216),
        id: S::CatchWait,
        animation: 244,
        anim: callbacks::animation::capture,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::catch,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_ThrowB = 220; ftData_MotionStateList[220].
    rows[S::ThrowB as usize] = MotionRow {
        action: ActionId(220),
        id: S::ThrowB,
        animation: 248,
        anim: callbacks::animation::throw,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CapturePulledLw = 226; ftData_MotionStateList[226].
    rows[S::CapturePulledLw as usize] = MotionRow {
        action: ActionId(226),
        id: S::CapturePulledLw,
        animation: 254,
        anim: callbacks::animation::capture,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::capture,
        collision: callbacks::collision::capture,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CaptureWaitLw = 227; ftData_MotionStateList[227].
    rows[S::CaptureWaitLw as usize] = MotionRow {
        action: ActionId(227),
        id: S::CaptureWaitLw,
        animation: 255,
        anim: callbacks::animation::capture,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::capture,
        collision: callbacks::collision::capture,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EscapeF = 233; ftData_MotionStateList[233].
    rows[S::EscapeF as usize] = MotionRow {
        action: ActionId(233),
        id: S::EscapeF,
        animation: 42,
        anim: callbacks::animation::escape,
        iasa: callbacks::input::escape,
        physics: callbacks::physics::escape,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EscapeB = 234; ftData_MotionStateList[234].
    rows[S::EscapeB as usize] = MotionRow {
        action: ActionId(234),
        id: S::EscapeB,
        animation: 43,
        anim: callbacks::animation::escape,
        iasa: callbacks::input::escape,
        physics: callbacks::physics::escape,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EscapeN = 235; ftData_MotionStateList[235].
    rows[S::EscapeN as usize] = MotionRow {
        action: ActionId(235),
        id: S::EscapeN,
        animation: 41,
        anim: callbacks::animation::escape,
        iasa: callbacks::input::escape_n,
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EscapeAir = 236; ftData_MotionStateList[236].
    rows[S::EscapeAir as usize] = MotionRow {
        action: ActionId(236),
        id: S::EscapeAir,
        animation: 44,
        anim: callbacks::animation::escape_air,
        iasa: callbacks::input::escape_air,
        physics: callbacks::physics::escape_air,
        collision: callbacks::collision::escape_air,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_ThrownB = 240; ftData_MotionStateList[240].
    rows[S::ThrownB as usize] = MotionRow {
        action: ActionId(240),
        id: S::ThrownB,
        animation: 263,
        anim: callbacks::animation::thrown,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::capture,
        collision: callbacks::collision::thrown,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Pass = 244; ftData_MotionStateList[244].
    rows[S::Pass as usize] = MotionRow {
        action: ActionId(244),
        id: S::Pass,
        animation: 209,
        anim: callbacks::animation::pass,
        iasa: callbacks::input::aerial,
        physics: callbacks::physics::pass,
        collision: callbacks::collision::pass,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CliffCatch = 252; ftData_MotionStateList[252].
    rows[S::CliffCatch as usize] = MotionRow {
        action: ActionId(252),
        id: S::CliffCatch,
        animation: 216,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_catch,
        physics: callbacks::physics::cliff_catch,
        collision: callbacks::collision::cliff_catch,
        camera: callbacks::camera::cliff,
        implemented: true,
    };
    // ftCo_MS_CliffWait = 253; ftData_MotionStateList[253].
    rows[S::CliffWait as usize] = MotionRow {
        action: ActionId(253),
        id: S::CliffWait,
        animation: 217,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_wait,
        physics: callbacks::physics::cliff_catch,
        collision: callbacks::collision::cliff_catch,
        camera: callbacks::camera::cliff,
        implemented: true,
    };
    // ftCo_MS_CliffClimbQuick = 255; ftData_MotionStateList[255].
    rows[S::CliffClimbQuick as usize] = MotionRow {
        action: ActionId(255),
        id: S::CliffClimbQuick,
        animation: 220,
        anim: callbacks::animation::cliff_climb,
        iasa: callbacks::input::cliff_climb,
        physics: callbacks::physics::cliff_climb,
        collision: callbacks::collision::cliff_climb,
        camera: callbacks::camera::cliff,
        implemented: true,
    };
    // ftCo_MS_CliffEscapeQuick = 259; ftData_MotionStateList[259].
    rows[S::CliffEscapeQuick as usize] = MotionRow {
        action: ActionId(259),
        id: S::CliffEscapeQuick,
        animation: 224,
        anim: callbacks::animation::cliff_climb,
        iasa: callbacks::input::cliff_climb,
        physics: callbacks::physics::cliff_climb,
        collision: callbacks::collision::cliff_climb,
        camera: callbacks::camera::cliff,
        implemented: true,
    };
    // ftCo_MS_CliffJumpSlow1 = 260; ftData_MotionStateList[260].
    rows[S::CliffJumpSlow1 as usize] = MotionRow {
        action: ActionId(260),
        id: S::CliffJumpSlow1,
        animation: 225,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_catch,
        physics: callbacks::physics::cliff_catch,
        collision: callbacks::collision::cliff_catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CliffJumpSlow2 = 261; ftData_MotionStateList[261].
    rows[S::CliffJumpSlow2 as usize] = MotionRow {
        action: ActionId(261),
        id: S::CliffJumpSlow2,
        animation: 226,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_catch,
        physics: callbacks::physics::cliff_jump2,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CliffJumpQuick1 = 262; ftData_MotionStateList[262].
    rows[S::CliffJumpQuick1 as usize] = MotionRow {
        action: ActionId(262),
        id: S::CliffJumpQuick1,
        animation: 227,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_catch,
        physics: callbacks::physics::cliff_catch,
        collision: callbacks::collision::cliff_catch,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_CliffJumpQuick2 = 263; ftData_MotionStateList[263].
    rows[S::CliffJumpQuick2 as usize] = MotionRow {
        action: ActionId(263),
        id: S::CliffJumpQuick2,
        animation: 228,
        anim: callbacks::animation::cliff_catch,
        iasa: callbacks::input::cliff_catch,
        physics: callbacks::physics::cliff_jump2,
        collision: callbacks::collision::jump,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Entry = 322; ftData_MotionStateList[322].
    rows[S::Entry as usize] = MotionRow {
        action: ActionId(322),
        id: S::Entry,
        animation: -1,
        anim: callbacks::animation::entry,
        iasa: callbacks::input::entry,
        physics: callbacks::physics::entry,
        collision: callbacks::collision::entry,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EntryStart = 323; ftData_MotionStateList[323].
    rows[S::EntryStart as usize] = MotionRow {
        action: ActionId(323),
        id: S::EntryStart,
        animation: 238,
        anim: callbacks::animation::entry,
        iasa: callbacks::input::entry,
        physics: callbacks::physics::entry,
        collision: callbacks::collision::entry,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_EntryEnd = 324; ftData_MotionStateList[324].
    rows[S::EntryEnd as usize] = MotionRow {
        action: ActionId(324),
        id: S::EntryEnd,
        animation: -1,
        anim: callbacks::animation::entry,
        iasa: callbacks::input::entry,
        physics: callbacks::physics::entry,
        collision: callbacks::collision::entry,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // S2: ftData_MotionStateList[65..74], aerials and directional landing lag.
    rows[S::AttackAirN as usize] = MotionRow {
        action: ActionId(65),
        id: S::AttackAirN,
        animation: 68,
        anim: crate::fighter::attack::aerial::animation,
        iasa: crate::fighter::attack::aerial::input,
        physics: callbacks::physics::fall,
        collision: crate::fighter::attack::aerial::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::LandingAirN as usize] = MotionRow {
        action: ActionId(70),
        id: S::LandingAirN,
        animation: 73,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::entry, // ftCo_LandingAir_IASA: empty.
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::AttackAirF as usize] = MotionRow {
        action: ActionId(66),
        id: S::AttackAirF,
        animation: 69,
        anim: crate::fighter::attack::aerial::animation,
        iasa: crate::fighter::attack::aerial::input,
        physics: callbacks::physics::fall,
        collision: crate::fighter::attack::aerial::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::LandingAirF as usize] = MotionRow {
        action: ActionId(71),
        id: S::LandingAirF,
        animation: 74,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::entry, // ftCo_LandingAir_IASA: empty.
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::AttackAirB as usize] = MotionRow {
        action: ActionId(67),
        id: S::AttackAirB,
        animation: 70,
        anim: crate::fighter::attack::aerial::animation,
        iasa: crate::fighter::attack::aerial::input,
        physics: callbacks::physics::fall,
        collision: crate::fighter::attack::aerial::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::LandingAirB as usize] = MotionRow {
        action: ActionId(72),
        id: S::LandingAirB,
        animation: 75,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::entry, // ftCo_LandingAir_IASA: empty.
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::AttackAirHi as usize] = MotionRow {
        action: ActionId(68),
        id: S::AttackAirHi,
        animation: 71,
        anim: crate::fighter::attack::aerial::animation,
        iasa: crate::fighter::attack::aerial::input,
        physics: callbacks::physics::fall,
        collision: crate::fighter::attack::aerial::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::LandingAirHi as usize] = MotionRow {
        action: ActionId(73),
        id: S::LandingAirHi,
        animation: 76,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::entry, // ftCo_LandingAir_IASA: empty.
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::AttackAirLw as usize] = MotionRow {
        action: ActionId(69),
        id: S::AttackAirLw,
        animation: 72,
        anim: crate::fighter::attack::aerial::animation,
        iasa: crate::fighter::attack::aerial::input,
        physics: callbacks::physics::fall,
        collision: crate::fighter::attack::aerial::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::LandingAirLw as usize] = MotionRow {
        action: ActionId(74),
        id: S::LandingAirLw,
        animation: 77,
        anim: callbacks::animation::landing,
        iasa: callbacks::input::entry, // ftCo_LandingAir_IASA: empty.
        physics: callbacks::physics::guard_on,
        collision: callbacks::collision::ground_wait,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // S5: prone recovery rows and state-specific wait/bounce callbacks.
    rows[S::DownWaitU as usize].iasa = crate::fighter::down::wait_input;
    rows[S::DownWaitD as usize].iasa = crate::fighter::down::wait_input;
    // ftData_MotionStateList[186]: ftCo_MS_DownStandU.
    rows[S::DownStandU as usize] = MotionRow {
        action: ActionId(186),
        id: S::DownStandU,
        animation: 186,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[187]: ftCo_MS_DownAttackU.
    rows[S::DownAttackU as usize] = MotionRow {
        action: ActionId(187),
        id: S::DownAttackU,
        animation: 187,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[188]: ftCo_MS_DownFowardU.
    rows[S::DownFowardU as usize] = MotionRow {
        action: ActionId(188),
        id: S::DownFowardU,
        animation: 188,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[189]: ftCo_MS_DownBackU.
    rows[S::DownBackU as usize] = MotionRow {
        action: ActionId(189),
        id: S::DownBackU,
        animation: 189,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[194]: ftCo_MS_DownStandD.
    rows[S::DownStandD as usize] = MotionRow {
        action: ActionId(194),
        id: S::DownStandD,
        animation: 194,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[195]: ftCo_MS_DownAttackD.
    rows[S::DownAttackD as usize] = MotionRow {
        action: ActionId(195),
        id: S::DownAttackD,
        animation: 195,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[196]: ftCo_MS_DownFowardD.
    rows[S::DownFowardD as usize] = MotionRow {
        action: ActionId(196),
        id: S::DownFowardD,
        animation: 196,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftData_MotionStateList[197]: ftCo_MS_DownBackD.
    rows[S::DownBackD as usize] = MotionRow {
        action: ActionId(197),
        id: S::DownBackD,
        animation: 197,
        anim: crate::fighter::down::recovery_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::jab,
        collision: callbacks::collision::escape,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // ftCo_MS_Ottotto = 245, ftCo_MS_OttottoWait = 246: teetering at a floor edge.
    rows[S::Ottotto as usize] = MotionRow {
        action: ActionId(245),
        id: S::Ottotto,
        animation: 210,
        anim: callbacks::animation::ottotto,
        iasa: callbacks::input::ottotto,
        physics: callbacks::physics::ottotto,
        collision: callbacks::collision::ottotto,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::OttottoWait as usize] = MotionRow {
        action: ActionId(246),
        id: S::OttottoWait,
        animation: 211,
        anim: callbacks::animation::ottotto_wait,
        iasa: callbacks::input::ottotto,
        physics: callbacks::physics::ottotto,
        collision: callbacks::collision::ottotto,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // S6: shield-break and dizzy rows, ftData_MotionStateList[205..211].
    rows[S::ShieldBreakFly as usize] = MotionRow {
        action: ActionId(205),
        id: S::ShieldBreakFly,
        animation: 286,
        anim: crate::fighter::shield_break::fly_animation,
        iasa: callbacks::input::catch,
        physics: crate::fighter::shield_break::fly_physics,
        collision: crate::fighter::shield_break::fly_collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::ShieldBreakDownU as usize] = MotionRow {
        action: ActionId(207),
        id: S::ShieldBreakDownU,
        animation: 288,
        anim: crate::fighter::shield_break::down_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::ShieldBreakStandU as usize] = MotionRow {
        action: ActionId(209),
        id: S::ShieldBreakStandU,
        animation: 290,
        anim: crate::fighter::shield_break::stand_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    rows[S::Furafura as usize] = MotionRow {
        action: ActionId(211),
        id: S::Furafura,
        animation: 205,
        anim: crate::fighter::shield_break::dizzy_animation,
        iasa: callbacks::input::catch,
        physics: callbacks::physics::down,
        collision: callbacks::collision::ground_action,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    // S7/S8: common throw pairs and quick ledge attack.
    rows[S::ThrowF as usize] = MotionRow {
        action: ActionId(S::ThrowF as u16),
        id: S::ThrowF,
        animation: 247,
        ..rows[S::ThrowB as usize]
    };
    rows[S::ThrowHi as usize] = MotionRow {
        action: ActionId(S::ThrowHi as u16),
        id: S::ThrowHi,
        animation: 249,
        ..rows[S::ThrowB as usize]
    };
    rows[S::ThrowLw as usize] = MotionRow {
        action: ActionId(S::ThrowLw as u16),
        id: S::ThrowLw,
        animation: 250,
        ..rows[S::ThrowB as usize]
    };
    rows[S::ThrownF as usize] = MotionRow {
        action: ActionId(S::ThrownF as u16),
        id: S::ThrownF,
        animation: 262,
        ..rows[S::ThrownB as usize]
    };
    rows[S::ThrownHi as usize] = MotionRow {
        action: ActionId(S::ThrownHi as u16),
        id: S::ThrownHi,
        animation: 264,
        ..rows[S::ThrownB as usize]
    };
    rows[S::ThrownLw as usize] = MotionRow {
        action: ActionId(S::ThrownLw as u16),
        id: S::ThrownLw,
        animation: 265,
        ..rows[S::ThrownB as usize]
    };
    rows[S::CliffAttackQuick as usize] = MotionRow {
        action: ActionId(S::CliffAttackQuick as u16),
        id: S::CliffAttackQuick,
        animation: 222,
        anim: crate::fighter::ledge::attack_animation,
        ..rows[S::CliffClimbQuick as usize]
    };
    rows[S::CatchWait as usize].iasa = crate::fighter::grab_escape::pummel_input;
    rows[S::CaptureWaitLw as usize].anim = crate::fighter::grab_escape::capture_animation;
    rows[S::CatchAttack as usize] = MotionRow {
        action: ActionId(217),
        id: S::CatchAttack,
        animation: 245,
        anim: crate::fighter::grab_escape::pummel_animation,
        ..rows[S::CatchPull as usize]
    };
    rows[S::CatchCut as usize] = MotionRow {
        action: ActionId(218),
        id: S::CatchCut,
        animation: 246,
        anim: crate::fighter::grab_escape::cut_animation,
        ..rows[S::CatchPull as usize]
    };
    rows[S::CaptureCut as usize] = MotionRow {
        action: ActionId(229),
        id: S::CaptureCut,
        animation: 257,
        anim: crate::fighter::grab_escape::cut_animation,
        physics: crate::fighter::grab_escape::cut_physics,
        ..rows[S::CatchCut as usize]
    };
    rows[S::CaptureDamageLw as usize] = MotionRow {
        action: ActionId(228),
        id: S::CaptureDamageLw,
        animation: 256,
        anim: crate::fighter::grab_escape::capture_damage_animation,
        ..rows[S::CaptureWaitLw as usize]
    };
    rows
}
