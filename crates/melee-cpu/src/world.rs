//! What the CPU reads outside its own fighter: the fighter list, the map,
//! the blast zones and the items.
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_types::Vec3;
use melee_ft::fighter::{life::Arena, Fighter};
use melee_mp::{CollMap, LineHit};
use melee_types::GrKind;

/// An item as the CPU's target choice sees it.
#[derive(Clone, Copy, Debug)]
pub struct ItemView {
    /// The scene's item id (stands in for the Item pointer).
    pub id: u32,
    pub kind: melee_types::ItemKind,
    pub position: Vec3,
    /// Item_IsGrabbable.
    pub grabbable: bool,
    /// it_8026C1B4 (it_80275870): one of its four hitboxes is active.
    pub hitbox_active: bool,
    /// x378_itemColl.floor.index.
    pub floor_line: i32,
}

impl ItemView {
    /// A placeholder for fixed arrays of views.
    pub const NONE: Self = Self {
        id: 0,
        kind: melee_types::ItemKind::Capsule,
        position: Vec3::ZERO,
        grabbable: false,
        hitbox_active: false,
        floor_line: -1,
    };
}

/// The scene around a thinking CPU.
pub struct Scene<'a> {
    /// Every fighter in fighter-list order; the thinking fighter's own entry
    /// is `None` (it is borrowed mutably).
    pub fighters: &'a [Option<&'a Fighter>],
    /// The thinking fighter's index in `fighters`.
    pub own: usize,
    pub map: &'a mut CollMap,
    /// Stage_GetBlastZone*Offset.
    pub arena: &'a Arena,
    /// HSD_GObj_Entities->items, in list order.
    pub items: &'a [ItemView],
    /// PlCo's CPU tables (Fighter_804D64FC).
    pub data: &'a crate::desc::CpuData,
    /// gm_8016C75C: the thinking fighter's player's KO total.
    pub player_kills: i32,
    /// p_ftCommonData->horizontal_stick_deadzone.
    pub horizontal_deadzone: f32,
}

impl Scene<'_> {
    /// The other fighters with their list indices, in list order.
    pub fn others(&self) -> impl Iterator<Item = (usize, &Fighter)> {
        self.fighters
            .iter()
            .enumerate()
            .filter_map(|(index, f)| f.map(|f| (index, f)))
    }
    pub fn fighter(&self, index: usize) -> &Fighter {
        self.fighters[index].expect("another fighter")
    }
    /// ftCo_800A589C (0x800A589C): the first other fighter of the same
    /// player, unless it is out of play (x221F_b3).
    pub fn partner_of(&self, me: &Fighter) -> Option<(usize, &Fighter)> {
        let (index, partner) = self
            .others()
            .find(|(_, f)| f.core.player.id == me.core.player.id)?;
        (!partner.core.status.disabled).then_some((index, partner))
    }
    /// The stage kind (stage_info.grkind).
    pub fn stage(&self) -> GrKind {
        self.map.grkind()
    }
    /// ftCo_800A1B38 (0x800A1B38): floors the CPU ignores (Big Blue's,
    /// Mushroom Kingdom's, Corneria's and Venom's moving floors).
    pub fn ignored_floor(&self, _line: i32) -> bool {
        match self.stage() {
            GrKind::BigBlue | GrKind::Inishie1 | GrKind::Corneria | GrKind::Venom => {
                unimplemented!("ftCo_800A1B38: grBigBlue_801EF844 and its siblings")
            }
            _ => false,
        }
    }
    /// mpCheckFloor with no line, joint or callback filter.
    pub fn check_floor(&mut self, ax: f32, ay: f32, bx: f32, by: f32) -> Option<LineHit> {
        self.map.check_floor(ax, ay, bx, by, 0.0, -1, -1, -1, None)
    }
    /// mpCheckFloor, then drop a floor the CPU ignores (the inline pattern
    /// of ftCo_800B33B0, ftCo_800A7AAC and others).
    pub fn check_usable_floor(&mut self, ax: f32, ay: f32, bx: f32, by: f32) -> Option<LineHit> {
        self.check_floor(ax, ay, bx, by)
            .filter(|hit| !self.ignored_floor(hit.line_id))
    }
    /// Whether `point` lies outside the blast zones shrunk by the CPU's
    /// half size (the inlined test of ftCo_800B33B0, inlineD0 and others):
    /// left + hw, right - hw, bottom + hh, top - hh, each an fadds/fsubs.
    pub fn outside(&self, x: f32, y: f32, half_size: [f32; 2]) -> bool {
        let [half_width, half_height] = half_size;
        x < half_width + self.arena.left
            || x > self.arena.right - half_width
            || y < half_height + self.arena.bottom
            || y > self.arena.top - half_height
    }
}

/// ftCo_800A1AB4 (0x800A1AB4) and its inlined copies: sqrtf of
/// dx*dx + dy*dy, the first square fused (800A1AD8: fmadds).
pub fn distance(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    sqrtf(fmadds(dx, dx, dy * dy))
}
