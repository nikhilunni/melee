//! grCastle_801CD37C (0x801CD37C): creation and callback order. Maps 0, 4,
//! 3, 6 at creation (grCastle_801CD4D0); the deferred grCastle_801CD610
//! makes the switches (20, 19, 18), each its block set (17, 7, 5); the
//! Bullet Bill's maps (8..16, then 2 and 1) come and go during the match.
use super::{
    bills::{Bill, BillTimer, Cannon},
    platforms::Platform,
    switches::{BlockSet, Switch, SwitchSelector},
    wind::Gusts,
    Parameters,
};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 4] = [0, 4, 3, 6];
/// The roof: the controller, the flags, the fountain.
pub const ROOF_MAP: u8 = 3;
pub const PLATFORM_MAP: u8 = 6;
/// grCastle_801CD8A8 (map 3): switches, Bill, Ground_801C2FE0,
/// lb_800115F4, the wind.
pub const DYNAMICS_PROC: u32 = 0x801C_D8A8;
/// grCastle_801CD37C's mpLib_80057BC0 calls: the block sets' joints start
/// disabled.
pub const DISABLED_JOINTS: [i32; 12] = [0, 1, 2, 6, 7, 8, 9, 10, 11, 12, 13, 14];
/// grCastle_801CD658: the fountain's generator, grLib_801C96F8(0x7536,
/// 0x1E) at this point times the map scale.
pub const FOUNTAIN: (u32, [f32; 3]) = (0x7536, [-257.0, 13.5, -252.0]);
/// grCastle_801D09B8: the moat's surface (times the map scale), its lift
/// on a falling fighter, and the splash (grCastle_801D0A9C).
pub const MOAT_LEVEL: f32 = 14.0;
pub const SPLASH_PARTICLE: u32 = 0x7530;

/// grCs_StageCallbacks' gobj procs by map.
pub fn map_callback(map: u8) -> Option<(&'static str, u32)> {
    Some(match map {
        0 => return None,
        1 => ("grCastle_801CE9E8", 0x801C_E9E8),
        2 => ("grCastle_801CE860", 0x801C_E860),
        3 => ("grCastle_801CD8A8", DYNAMICS_PROC),
        4 => ("grCastle_801CD608", 0x801C_D608),
        5 | 7 | 17 => ("grCastle_801CEF04", 0x801C_EF04),
        6 => ("grCastle_801CDC44", 0x801C_DC44),
        8..=16 => ("grCastle_801CE578", 0x801C_E578),
        18..=20 => ("grCastle_801CF308", 0x801C_F308),
        _ => unreachable!("castle map {map}"),
    })
}

/// The explosion (map 1) while it lives.
#[derive(Clone, Debug)]
pub struct Explosion {
    pub subject: Option<melee_cm::Subject>,
    pub item: Option<u32>,
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct CastleStage {
    pub parameters: Parameters,
    pub platforms: [Platform; 2],
    pub gusts: Gusts,
    /// Set by the deferred grCastle_801CD610.
    pub selector: Option<SwitchSelector>,
    pub bill_timer: Option<BillTimer>,
    /// By [`super::switches::SWITCH_MAPS`] order.
    pub switches: [Switch; 3],
    pub switch_items: [Option<u32>; 3],
    /// By [`super::switches::BLOCK_MAPS`] order (each its switch's).
    pub blocks: [Option<BlockSet>; 3],
    pub bill: Option<Bill>,
    pub bill_item: Option<u32>,
    pub bill_subject: Option<melee_cm::Subject>,
    pub cannon: Option<Cannon>,
    pub explosion: Option<Explosion>,
    /// ftCo_800C06E8(gobj, 6, grCastle_801D09B8): registered at the start.
    pub moat_active: bool,
}

impl CastleStage {
    pub fn new(parameters: Parameters) -> Self {
        Self {
            platforms: [Platform::new(&parameters); 2],
            parameters,
            gusts: Gusts::default(),
            selector: None,
            bill_timer: None,
            switches: [Switch::new(); 3],
            switch_items: [None; 3],
            blocks: [None; 3],
            bill: None,
            bill_item: None,
            bill_subject: None,
            cannon: None,
            explosion: None,
            moat_active: false,
        }
    }

    /// The switch whose material item `item` is.
    pub fn switch_of_item(&self, item: u32) -> Option<usize> {
        self.switch_items.iter().position(|&i| i == Some(item))
    }

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
        for map in MAP_ORDER {
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            if let Some((name, address)) = map_callback(map) {
                rows.push(row(4, 5, Some(map), name, address));
            }
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
