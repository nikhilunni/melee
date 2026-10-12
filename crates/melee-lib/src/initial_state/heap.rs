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

/// One recorded case: the stage, each fighter created before the partner
/// with the costumes seen (a bit per costume), and Nana's Fighter address.
struct BootAddress {
    stage: G,
    before: &'static [(K, u8)],
    nana: u32,
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
static NANA_AFTER_BOOT: &[BootAddress] = &[
    BootAddress { stage: G::Battle, before: &[(K::Popo, costumes(&[0]))], nana: 0x80C8_E1C0 },
    BootAddress { stage: G::OldPupupu, before: &[(K::Popo, costumes(&[2]))], nana: 0x80C9_3FE0 },
    BootAddress { stage: G::PStadium, before: &[(K::Popo, costumes(&[0, 1]))], nana: 0x80C9_66E0 },
    BootAddress { stage: G::Last, before: &[(K::Popo, costumes(&[0]))], nana: 0x80C9_D2A0 },
    BootAddress { stage: G::Story, before: &[(K::Popo, costumes(&[0, 1]))], nana: 0x80C9_E340 },
    BootAddress { stage: G::Story, before: &[(K::Popo, costumes(&[3]))], nana: 0x80C9_E380 },
    BootAddress { stage: G::Izumi, before: &[(K::Popo, costumes(&[1]))], nana: 0x80CA_F520 },
    BootAddress { stage: G::Story, before: &[(K::Purin, costumes(&[0, 4])), (K::Popo, costumes(&[0, 1]))], nana: 0x80CC_D940 },
    BootAddress { stage: G::Battle, before: &[(K::Luigi, costumes(&[2])), (K::Popo, costumes(&[0]))], nana: 0x80CC_9E40 },
    BootAddress { stage: G::Battle, before: &[(K::Falco, costumes(&[3])), (K::Popo, costumes(&[1]))], nana: 0x80CC_ED60 },
    BootAddress { stage: G::Story, before: &[(K::Falco, costumes(&[0])), (K::Popo, costumes(&[0]))], nana: 0x80CD_F300 },
    BootAddress { stage: G::Last, before: &[(K::Captain, costumes(&[3])), (K::Popo, costumes(&[0]))], nana: 0x80CE_1700 },
    BootAddress { stage: G::Izumi, before: &[(K::Captain, costumes(&[1])), (K::Popo, costumes(&[0]))], nana: 0x80CF_3840 },
];

/// The Fighter address of the roster's `index`th fighter after boot, where
/// a recording shows it: so far only a partner's (the one fighter whose
/// address the game reads as data).
pub(super) fn fighter_address_after_boot(setup: &Setup, index: usize) -> Option<u32> {
    let roster = setup.roster();
    if roster[index].descriptor.kind != K::Nana {
        return None;
    }
    let before: Vec<(K, u8)> = roster[..index]
        .iter()
        .map(|entry| (entry.descriptor.kind, setup.fighters[entry.player].costume))
        .collect();
    nana_after_boot(setup.stage.kind, &before)
}

/// Nana's address given the stage and the kind and costume of each fighter
/// created before her.
fn nana_after_boot(stage: G, before: &[(K, u8)]) -> Option<u32> {
    NANA_AFTER_BOOT
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
        .map(|row| row.nana)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nana_has_an_address_only_where_a_recording_shows_one() {
        type Case = (G, &'static [(K, u8)], Option<u32>);
        let cases: [Case; 5] = [
            (G::Last, &[(K::Popo, 0)], Some(0x80C9_D2A0)),
            (G::Story, &[(K::Popo, 3)], Some(0x80C9_E380)),
            (
                G::Izumi,
                &[(K::Captain, 1), (K::Popo, 0)],
                Some(0x80CF_3840),
            ),
            // A costume and a first fighter no boundary was made with.
            (G::Last, &[(K::Popo, 2)], None),
            (G::Last, &[(K::Fox, 0), (K::Popo, 0)], None),
        ];
        for (stage, before, address) in cases {
            assert_eq!(
                nana_after_boot(stage, before),
                address,
                "{stage:?} {before:?}"
            );
        }
    }
}
