mod support;
use ft_drmario::{
    attributes::{read_attributes, MarioAttributes},
    init::DrMario,
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};
use support::{archive, word};

fn attributes() -> Vec<u8> {
    let mut data = vec![0; 0x8C];
    word(&mut data, 0x14, 0x54); // It_Kind_DrMario_Cape
    data
}

fn drmario() -> DrMario {
    DrMario::new(MarioAttributes::read(&archive(&attributes(), &[], None), 0).unwrap())
}

#[test]
fn attributes_come_from_the_drmario_root_with_the_sheet_kind() {
    // ftData at +84 has a relocated ext_attr pointer to offset zero.
    let data = attributes();
    let source = archive(&data, &[0x88], Some(("ftDataDrmario", 0x84)));
    let attrs = read_attributes(&source, "ftDataDrmario").unwrap();
    assert_eq!(attrs.cape.cape_kind, ItemKind::DrMarioSheet);
    assert!(read_attributes(&source, "ftDataMario").is_err());
}

#[test]
fn load_enables_every_special_but_not_the_wall_jump() {
    let mut drmario = drmario();
    let mut capabilities = Capabilities::default();
    drmario.on_load(&mut capabilities);
    assert_eq!(drmario.kind(), FighterKind::DrMario);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
}

#[test]
fn death_keeps_the_vitamin_colours_and_landing_clears_the_aerial_flags() {
    let mut drmario = drmario();
    let mut raw = vec![0; 0x2244];
    word(&mut raw, 0x222C, 3);
    word(&mut raw, 0x2230, 5);
    word(&mut raw, 0x2234, 1);
    word(&mut raw, 0x2238, 1);
    drmario.restore_saved(&raw);
    let s = &drmario.specials;
    assert_eq!((s.vitamin_current, s.vitamin_previous), (3, 5));
    assert!(s.tornado_charged && s.cape_boosted);
    drmario.on_landing(false);
    assert!(!drmario.specials.tornado_charged && !drmario.specials.cape_boosted);
    drmario.restore_saved(&raw);
    drmario.model_group = 2;
    drmario.on_reset();
    let s = &drmario.specials;
    assert_eq!((s.vitamin_current, s.vitamin_previous), (3, 5));
    assert!(!s.tornado_charged && !s.cape_boosted);
    assert_eq!(drmario.model_group, 0);
}
