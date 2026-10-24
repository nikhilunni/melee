//! Where retail's heap puts a Fighter, for the scenes that know it.
//!
//! HSD_ObjAlloc (0x8037ABC8) gives each Fighter its block, and one branch
//! of the game reads the block's address as data: ftCo_800AC5A0 passes the
//! low byte of the Fighter pointer as the CPU's stick y (the partner's
//! steer in hitlag with no knockback, `melee-cpu` `tumble.rs`). A savestate
//! and a boundary recording carry the address. A scene built from
//! parameters alone takes the address a console booted straight into the
//! match gives, where a retail recording shows it: every boundary
//! `make_boundary.py` makes reaches its match the same way, and the first
//! fighter's block then depends on the stage alone
//! (docs/PORT_NOTES/STALE_HEAP.md). A later fighter's block depends on what
//! the fighters before it loaded, so the table below is keyed by them and
//! holds only what the recordings show; any other scene has no address.
use crate::setup::Setup;
use melee_types::{FighterKind as K, GrKind as G};

/// One recorded case: the stage, each fighter created before this one
/// with the costumes seen (a bit per costume), and its Fighter address.
struct BootAddress {
    stage: G,
    before: &'static [(K, u8)],
    address: u32,
}

const fn costumes(list: &[u8]) -> u8 {
    let mut mask = 0;
    let mut i = 0;
    while i < list.len() {
        mask |= 1 << list[i];
        i += 1;
    }
    mask
}

/// Nana's Fighter address in the first record of the `start_*` recordings
/// (2026-10-12: 23 Ice Climbers boundaries and the scenes from them).
/// With Popo the first fighter she sits 0x31940 past him on five stages
/// whatever the opponent is; Yoshi's Story differs, and by costume.
#[rustfmt::skip]
static AFTER_BOOT: &[BootAddress] = &[
    BootAddress { stage: G::Battle, before: &[(K::Popo, costumes(&[0]))], address: 0x80C8_E1C0 },
    BootAddress { stage: G::OldPupupu, before: &[(K::Popo, costumes(&[2]))], address: 0x80C9_3FE0 },
    BootAddress { stage: G::PStadium, before: &[(K::Popo, costumes(&[0, 1]))], address: 0x80C9_66E0 },
    BootAddress { stage: G::Last, before: &[(K::Popo, costumes(&[0]))], address: 0x80C9_D2A0 },
    BootAddress { stage: G::Story, before: &[(K::Popo, costumes(&[0, 1]))], address: 0x80C9_E340 },
    BootAddress { stage: G::Story, before: &[(K::Popo, costumes(&[3]))], address: 0x80C9_E380 },
    BootAddress { stage: G::Izumi, before: &[(K::Popo, costumes(&[1]))], address: 0x80CA_F520 },
    BootAddress { stage: G::Story, before: &[(K::Purin, costumes(&[0, 4])), (K::Popo, costumes(&[0, 1]))], address: 0x80CC_D940 },
    BootAddress { stage: G::Battle, before: &[(K::Luigi, costumes(&[2])), (K::Popo, costumes(&[0]))], address: 0x80CC_9E40 },
    BootAddress { stage: G::Battle, before: &[(K::Falco, costumes(&[3])), (K::Popo, costumes(&[1]))], address: 0x80CC_ED60 },
    BootAddress { stage: G::Story, before: &[(K::Falco, costumes(&[0])), (K::Popo, costumes(&[0]))], address: 0x80CD_F300 },
    BootAddress { stage: G::Last, before: &[(K::Captain, costumes(&[3])), (K::Popo, costumes(&[0]))], address: 0x80CE_1700 },
    BootAddress { stage: G::Izumi, before: &[(K::Captain, costumes(&[1])), (K::Popo, costumes(&[0]))], address: 0x80CF_3840 },
    // A person's Fox (costume 0) on port 1 before a second player of any
    // kind (2026-10-24: the 28 `start_*_fox_cpu*` boundaries).
    BootAddress { stage: G::Last, before: &[(K::Fox, costumes(&[0]))], address: 0x80CB_0980 },
    BootAddress { stage: G::Battle, before: &[(K::Fox, costumes(&[0]))], address: 0x80CA_18A0 },
    BootAddress { stage: G::Last, before: &[(K::Fox, costumes(&[0])), (K::Popo, costumes(&[0]))], address: 0x80CE_2280 },
    // The second player after a first of another kind (2026-10-24).
    BootAddress { stage: G::Last, before: &[(K::Mars, costumes(&[0]))], address: 0x80CB_7840 },
    BootAddress { stage: G::Battle, before: &[(K::Falco, costumes(&[0]))], address: 0x80C9_D460 },
    BootAddress { stage: G::Last, before: &[(K::Popo, costumes(&[0])), (K::Nana, costumes(&[0]))], address: 0x80CC_E780 },
    BootAddress { stage: G::Battle, before: &[(K::Seak, costumes(&[0])), (K::Zelda, costumes(&[0]))], address: 0x80CE_7320 },
];

/// The first fighter's block, which depends on the stage alone
/// (docs/PORT_NOTES/STALE_HEAP.md: 174 boundaries).
fn first_after_boot(stage: G) -> Option<u32> {
    Some(match stage {
        G::Battle => 0x80C5_C800,
        G::OldPupupu => 0x80C6_2660,
        G::PStadium => 0x80C6_4DE0,
        G::Last => 0x80C6_B960,
        G::Story => 0x80C6_CE60,
        G::Izumi => 0x80C7_DC20,
        _ => return None,
    })
}

/// The Fighter address of the roster's `index`th fighter after boot, where
/// a recording shows it (the game reads it as data only for a CPU: a
/// partner, or a CPU player).
pub(super) fn fighter_address_after_boot(setup: &Setup, index: usize) -> Option<u32> {
    let roster = setup.roster();
    if index == 0 {
        return first_after_boot(setup.stage.kind);
    }
    let before: Vec<(K, u8)> = roster[..index]
        .iter()
        .map(|entry| (entry.descriptor.kind, setup.fighters[entry.player].costume))
        .collect();
    after_boot(setup.stage.kind, &before)
}

/// A fighter's address given the stage and the kind and costume of each
/// fighter created before it.
fn after_boot(stage: G, before: &[(K, u8)]) -> Option<u32> {
    AFTER_BOOT
        .iter()
        .find(|row| {
            row.stage == stage
                && row.before.len() == before.len()
                && row
                    .before
                    .iter()
                    .zip(before)
                    .all(|(&(kind, seen), &(is, costume))| {
                        kind == is && costume < 8 && seen & (1 << costume) != 0
                    })
        })
        .map(|row| row.address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nana_has_an_address_only_where_a_recording_shows_one() {
        type Case = (G, &'static [(K, u8)], Option<u32>);
        let cases: [Case; 7] = [
            (G::Last, &[(K::Popo, 0)], Some(0x80C9_D2A0)),
            (G::Story, &[(K::Popo, 3)], Some(0x80C9_E380)),
            (
                G::Izumi,
                &[(K::Captain, 1), (K::Popo, 0)],
                Some(0x80CF_3840),
            ),
            // A costume and a first fighter no boundary was made with.
            (G::Last, &[(K::Popo, 2)], None),
            (G::Last, &[(K::Fox, 0), (K::Popo, 0)], Some(0x80CE_2280)),
            (G::Last, &[(K::Fox, 0)], Some(0x80CB_0980)),
            (G::Last, &[(K::Fox, 1)], None),
        ];
        for (stage, before, address) in cases {
            assert_eq!(
                after_boot(stage, before),
                address,
                "{stage:?} {before:?}"
            );
        }
    }
}
