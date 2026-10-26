//! grPura_80211D00 (0x80211D00): Ground creation and callback order. Maps
//! 0, 1 and 4; map 4's on_init (stageGObj4_OnInit) creates every Pokémon
//! (grPura_802125F0) before its own proc is registered. Scheduler keys are
//! the map ids, except the second and third map-13 Pokémon.
use super::{Pura, FLOATERS};
use crate::last::procs::ProcRegistration;

/// The sky (stageGObj1).
pub const SKY_MAP: u8 = 1;
/// The sequence (stageGObj4).
pub const SEQUENCE_MAP: u8 = 4;
/// Keys of the second and third map-13 Pokémon.
pub const EXTRA_KEYS: [u8; 2] = [28, 29];

/// The Pokémon's scheduler keys, in creation order.
pub const FLOATER_KEYS: [u8; 26] = {
    let mut keys = [0; 26];
    let mut i = 0;
    let mut extra = 0;
    while i < FLOATERS.len() {
        let map = FLOATERS[i].0;
        keys[i] = if i > 0 && FLOATERS[i - 1].0 == map {
            extra += 1;
            EXTRA_KEYS[extra - 1]
        } else {
            map
        };
        i += 1;
    }
    keys
};

/// GObj creation order.
pub const OBJECT_ORDER: [u8; 29] = {
    let mut order = [0; 29];
    order[0] = 0;
    order[1] = SKY_MAP;
    order[2] = SEQUENCE_MAP;
    let mut i = 0;
    while i < FLOATER_KEYS.len() {
        order[3 + i] = FLOATER_KEYS[i];
        i += 1;
    }
    order
};

/// Model (map id) behind a scheduler key.
pub fn model(key: u8) -> u8 {
    match key {
        28 | 29 => 13,
        _ => key,
    }
}

/// stage_callbacks gobj procs (s_link 4).
pub fn map_callback(key: u8) -> (&'static str, u32) {
    match key {
        0 => ("stageGObj0_GObjProc", 0x8021_1F24),
        SKY_MAP => ("stageGObj1_GObjProc", 0x8021_20E0),
        SEQUENCE_MAP => ("stageGObj4_GObjProc", SEQUENCE_PROC),
        _ => ("stageGObj2_GObjProc", FLOATER_PROC),
    }
}
/// stageGObj4_GObjProc: the subjects, the collision, the vertices,
/// mpLib_80055E24 and lb_800115F4.
pub const SEQUENCE_PROC: u32 = 0x8021_1FD8;
/// stageGObj2_GObjProc: a Pokémon follows its bone.
pub const FLOATER_PROC: u32 = 0x8021_231C;

impl Pura {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        let row = |s_link, p_link, map_id, callback, address| ProcRegistration {
            s_link,
            p_link,
            p_priority: 0,
            map_id,
            callback,
            address,
        };
        let mut rows = vec![
            row(0, 3, None, "Ground_801C461C", 0x801C461C),
            row(0, 4, None, "fn_801CADBC", 0x801CADBC),
        ];
        // Ground_GetStageGObj registers each GObj's s_link 1 and 4 procs;
        // setupStageCallbacks its gobj proc after its on_init, so map 4's
        // follows every Pokémon's.
        for key in OBJECT_ORDER {
            rows.push(row(1, 5, Some(key), "Ground_801C1CD0", 0x801C1CD0));
            rows.push(row(4, 5, Some(key), "Ground_801C1D38", 0x801C1D38));
            if key != SEQUENCE_MAP {
                let (name, address) = map_callback(key);
                rows.push(row(4, 5, Some(key), name, address));
            }
        }
        let (name, address) = map_callback(SEQUENCE_MAP);
        rows.push(row(4, 5, Some(SEQUENCE_MAP), name, address));
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_13_is_created_three_times_under_its_own_keys() {
        assert_eq!(&FLOATER_KEYS[9..14], &[12, 13, 28, 29, 14]);
        assert!(FLOATER_KEYS.iter().all(|&k| k != 4 && k != 1 && k != 0));
        assert_eq!(model(29), 13);
    }
}
