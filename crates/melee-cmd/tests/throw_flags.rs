use melee_cmd::{decode::decode, Command};

#[test]
fn retail_throw_flags_distinguish_release_and_reversal() {
    // ftAction_800718A4: these authored operands occupy the low 26 bits.
    assert!(matches!(
        decode(&[0x5000_0000], None, 1),
        Ok(Command::GrabRelease)
    ));
    assert!(matches!(
        decode(&[0x5000_0001], None, 1),
        Ok(Command::ThrowReverse)
    ));
    assert!(decode(&[0x5080_0000], None, 1).is_err());
}
