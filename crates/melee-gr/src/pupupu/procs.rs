//! grOldPupupu_802107E0 / 802108B4: Ground creation and callback order.
use super::Pupupu;
use crate::last::procs::ProcRegistration;
pub const MAP_ORDER: [u8; 8] = [0, 3, 7, 5, 4, 6, 1, 8];
impl Pupupu {
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
        for map in MAP_ORDER {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        for (map, name, address) in [
            (0, "grOldPupupu_802109D0", 0x802109D0),
            (3, "grOldPupupu_80211194", 0x80211194),
            (7, "grOldPupupu_802113E0", 0x802113E0),
            (5, "grOldPupupu_80210BC0", 0x80210BC0),
            (4, "grOldPupupu_80210B50", 0x80210B50),
            (6, "grOldPupupu_80211C1C", 0x80211C1C),
            (1, "grOldPupupu_80210A24", 0x80210A24),
            (8, "grOldPupupu_80210D10", 0x80210D10),
        ] {
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
