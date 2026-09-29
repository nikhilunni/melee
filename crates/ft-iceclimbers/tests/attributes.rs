mod support;
use ft_iceclimbers::{
    attributes::{read, IceClimberAttributes},
    init::{Climber, ClimberVars, IceClimber},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

fn attributes() -> IceClimberAttributes {
    let mut data = vec![0; 0x15C];
    word(&mut data, 0x00, 5.0_f32.to_bits());
    word(&mut data, 0xC4, (-5.0_f32).to_bits());
    word(&mut data, 0xC8, 5.0_f32.to_bits());
    IceClimberAttributes::read(&archive(&data, &[], None), 0).unwrap()
}

#[test]
fn relocated_attributes_name_the_spawn_offsets_and_nanas_armor() {
    let mut data = vec![0; 0x164];
    word(&mut data, 0x00, 5.0_f32.to_bits());
    word(&mut data, 0xC4, (-5.0_f32).to_bits());
    word(&mut data, 0xC8, 0x7FC0_1234);
    word(&mut data, 0x28, 1.5_f32.to_bits());
    word(&mut data, 0x68, 6);
    word(&mut data, 0x12C, 30.0_f32.to_bits());
    // ftData at +15C has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x160], Some(("ftDataNana", 0x15C)));
    let attrs = read(&source, "ftDataNana").unwrap();
    assert_eq!(attrs.leader_spawn_offset, 5.0);
    assert_eq!(attrs.partner_spawn_offset, -5.0);
    assert_eq!(attrs.partner_armor.to_bits(), 0x7FC0_1234);
    // The Squall Hammer's entry speed, press interval (an int) and Nana's
    // FallSpecial lag.
    assert_eq!(attrs.squall.ground_entry_speed, 1.5);
    assert_eq!(attrs.squall.press_interval, 6);
    assert_eq!(attrs.partner_squall_landing_lag, 30.0);
    assert!(read(&source, "ftDataPopo").is_err());
    assert!(IceClimberAttributes::read(&archive(&data[..0x15B], &[], None), 0).is_err());
}

#[test]
fn popo_leads_and_nana_follows_as_a_cpu_with_armor() {
    let mut popo = IceClimber::new(Climber::Popo, attributes());
    let mut capabilities = Capabilities::default();
    popo.on_load(&mut capabilities);
    assert_eq!(popo.kind(), FighterKind::Popo);
    assert!(capabilities.leads_partner && !capabilities.cpu_partner);
    assert_eq!(capabilities.spawn_offset, 5.0);
    assert_eq!(capabilities.armor, 0.0);
    assert_eq!(capabilities.specials, [true; 4]);

    let mut nana = IceClimber::new(Climber::Nana, attributes());
    let mut capabilities = Capabilities::default();
    nana.on_load(&mut capabilities);
    assert_eq!(nana.kind(), FighterKind::Nana);
    assert!(capabilities.cpu_partner && capabilities.refuses_healing_items);
    assert_eq!(capabilities.spawn_offset, -5.0);
    assert_eq!(capabilities.armor, 5.0);
    // ftData_SpecialS/Hi are NULL for Nana; N and Lw are Popo's.
    assert_eq!(capabilities.specials, [false, false, true, true]);
}

#[test]
fn landing_and_death_clear_the_climber_vars() {
    let mut popo = IceClimber::new(Climber::Popo, attributes());
    let mut raw = vec![0; 0x2254];
    raw[0x2230] = 0x80;
    word(&mut raw, 0x2234, 3);
    word(&mut raw, 0x224C, 7);
    word(&mut raw, 0x2250, 2.5_f32.to_bits());
    popo.restore_saved(&raw);
    assert_eq!(
        popo.vars,
        ClimberVars {
            model_groups: [0; 2],
            x2234: 3,
            x2230_b0: true,
            air_ice_shot_used: true,
            ice_drop: 2.5,
            ..ClimberVars::default()
        }
    );
    popo.on_landing(true);
    assert!(!popo.vars.air_ice_shot_used);
    popo.on_reset();
    assert_eq!(popo.vars, ClimberVars::default());
}
