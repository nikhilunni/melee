//! Standalone guest instruction results; no game or host-generated expectations.
//! Interpreter, FPSCR=0. ARM64 JIT agrees except 18 scaled subnormal inputs,
//! where it flushes the multiplier to zero. All estimate cases agree.
use gekko_math::fma::fmuls;

#[test]
fn fmuls_matches_interpreter_guest_probe() {
    let rows = include_str!("data/fmuls_probe.txt");
    assert_eq!(rows.lines().count(), 264);
    for (index, line) in rows.lines().enumerate() {
        let words: Vec<u64> = line
            .split_whitespace()
            .map(|word| u64::from_str_radix(word, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 7);
        let a = f64::from_bits(words[0]);
        let c = f64::from_bits(words[1]);
        let estimate = f64::from_bits(words[4]);
        for (actual, expected) in [
            (fmuls(a, c), words[2]),
            (fmuls(c, a), words[3]),
            (fmuls(estimate, estimate), words[5]),
            (fmuls(estimate, 0.5), words[6]),
        ] {
            assert_eq!(u64::from(actual.to_bits()), expected, "case {index}");
        }
    }
}
