use gekko_math::HsdRng;
use melee_ft::fighter::CpuState;

#[test]
fn attack_delay_truncates_before_mode_seven_halving() {
    // Second Randf is 0x5B3F / 65536. Retail ftCo_800B9704 uses two
    // fmadds (800B9734/800B974C), then fctiwz (800B9750).
    for (mode, level, delay) in [(4, 1, 193), (7, 1, 96), (4, 9, 30), (7, 9, 15)] {
        let mut rng = HsdRng::new(0x1234_5678);
        let cpu = CpuState::initialize(mode, level, &mut rng);
        assert_eq!(cpu.reaction_timer, 7);
        assert_eq!(cpu.attack_delay, delay, "mode {mode}, level {level}");
        assert_eq!(rng.seed, 0x5B3F_58B2);
    }
}
