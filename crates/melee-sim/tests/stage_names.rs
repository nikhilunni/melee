//! Every Versus stage is named the same in scenarios, Slippi replays and
//! the port, and an unported one fails closed naming the stage.
use melee_lib::Stage;

#[test]
fn every_versus_stage_has_its_slippi_name_and_st_kind() {
    for stage in Stage::VERSUS {
        assert_eq!(
            slp::ids::stage_name(u16::from(stage.st_kind())),
            Some(stage.name())
        );
        assert_eq!(Stage::from_name(stage.name()), Some(stage));
        assert_eq!(Stage::from_st_kind(u16::from(stage.st_kind())), Some(stage));
    }
    // Akaneia (0x15) and Icetop (0x1A) are not on the stage select screen.
    assert_eq!(Stage::from_st_kind(0x15), None);
    assert_eq!(Stage::from_st_kind(0x1A), None);
}

#[test]
fn ported_stages_are_versus_stages() {
    for stage in Stage::PORTED {
        assert!(Stage::VERSUS.contains(&stage));
        assert!(stage.is_ported(), "{stage:?}");
    }
}

#[test]
fn an_unported_stage_names_itself() {
    let unported = Stage::VERSUS
        .into_iter()
        .filter(|s| !s.is_ported())
        .collect::<Vec<_>>();
    assert!(!unported.is_empty());
    for stage in unported {
        let message = melee_lib::unported_stage(stage);
        assert!(message.contains(stage.display_name()), "{message}");
        assert!(message.contains(stage.name()), "{message}");
    }
}
