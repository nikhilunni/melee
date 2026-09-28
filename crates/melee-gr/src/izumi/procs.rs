//! grIzumi_801CBB88 (0x801CBB88): Ground creation and callback order.
//!
//! OnInit creates maps 0, 1 and 3 through grIzumi_801CBCE8. Map 3's
//! on_init (grIzumi_801CBE64) runs between its Ground procs and its gobj
//! proc and creates, in order, the star model (Ground_801C1A20, map -1),
//! map 2, and the two map-4 platforms. Scheduler keys are the map ids,
//! except the star and the second platform, which have their own.
use super::Izumi;
use crate::last::procs::ProcRegistration;

/// Key of the star GObj (Ground_801C1A20 with map id -1).
pub const STAR: u8 = 13;
/// Keys of the platforms: the left one is map 4 itself, the right one a
/// second instance of map 4.
pub const PLATFORMS: [u8; 2] = [4, 14];
/// GObj creation order in the Ground p_link.
pub const OBJECT_ORDER: [u8; 7] = [0, 1, 3, STAR, 2, PLATFORMS[0], PLATFORMS[1]];

/// Model (map id) behind a scheduler key.
pub fn model(key: u8) -> Option<u8> {
    match key {
        STAR => None,
        14 => Some(4),
        _ => Some(key),
    }
}

/// Scheduler key of a saved Ground: its map id, except the star (map -1)
/// and the platform moving collision joint 1.
pub fn saved_key(map: i32, collision_joint: i16) -> Option<u8> {
    match (map, collision_joint) {
        (-1, _) => Some(STAR),
        (4, joint) => PLATFORMS.get(usize::try_from(joint).ok()?).copied(),
        (map, _) => u8::try_from(map).ok(),
    }
}

/// grIz_StageCallbacks gobj procs (s_link 4).
pub fn map_callback(key: u8) -> (&'static str, u32) {
    match key {
        0 => ("grIzumi_801CBE08", 0x801CBE08),
        1 => ("grIzumi_801CBE5C", 0x801CBE5C),
        2 => ("grIzumi_801CCB10", 0x801CCB10),
        3 => ("grIzumi_801CC0D4", 0x801CC0D4),
        4 | 14 => ("grIzumi_801CC358", 0x801CC358),
        _ => unreachable!("Fountain of Dreams key {key}"),
    }
}

impl Izumi {
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
        for key in OBJECT_ORDER {
            rows.push(row(1, 5, Some(key), "Ground_801C1CD0", 0x801C1CD0));
        }
        for key in OBJECT_ORDER {
            rows.push(row(4, 5, Some(key), "Ground_801C1D38", 0x801C1D38));
            if key != STAR {
                let (name, address) = map_callback(key);
                rows.push(row(4, 5, Some(key), name, address));
            }
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
