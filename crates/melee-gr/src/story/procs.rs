//! grStory_801E3030 (0x801E3030): map creation order 0, 1, 3, 2.
use super::Story;
use crate::last::procs::ProcRegistration;
impl Story {
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
        for map in [0, 1, 3, 2] {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        for (map, name, address) in [
            (1, "grStory_801E322C", 0x801E322C),
            (3, "grStory_801E3334", 0x801E3334),
            (2, "grStory_801E33E0", 0x801E33E0),
        ] {
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
