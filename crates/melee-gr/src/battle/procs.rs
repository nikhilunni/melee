//! `grBattle_OnInit` (0x80219CA4) creates maps 0, 3, 1, 6 in that order.
use super::Battlefield;
use crate::last::procs::ProcRegistration;
impl Battlefield {
    /// `Ground_GetStageGObj` (0x801C14D0), `grBattle_80219D84` (0x80219D84),
    /// `Ground_801C466C` (0x801C466C), `grBattle_OnStart` (0x80219D58).
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
        for map in [0, 3, 1, 6] {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        for (map, callback, address) in [
            (0, "grBattle_GObj0_Callback2", 0x8021A114),
            (3, "grBattle_BG_Callback2", 0x8021A3BC),
            (1, "grBattle_GObj1_Callback2", 0x8021A26C),
            (6, "grBattle_GObj6_Callback2", 0x8021A174),
        ] {
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), callback, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
