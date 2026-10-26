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
    /// ip->owner's player slot when a fighter owns it.
    pub owner: Option<u8>,
    /// x5D4_hitboxes, as ftCo_800BB220 reads them.
    pub hitboxes: [Option<ItemHitView>; 4],
    /// msid, the item's state.
    pub motion: u16,
    /// xD44_lifeTimer.
    pub life_timer: f32,
    /// it_8026B960's Bob-omb case (`ItemLogic::fuse`): xDEC unless
    /// exploding, else -1.
    pub bomb_fuse: f32,
}

/// One of an item's hit capsules (x5D4_hitboxes[i].hit) as the CPU's
/// defence sees it.
#[derive(Clone, Copy, Debug)]
pub struct ItemHitView {
    /// The state is past HitCapsule_Enabled: it has positions.
    pub placed: bool,
    pub element: melee_types::HitElement,
    /// x4C and x58.
    pub position: Vec3,
    pub previous_position: Vec3,
    /// hit->scale, the capsule's radius.
    pub radius: f32,
    /// x42_b0.
    pub absorbable: bool,
    /// lbColl_8000ACFC: the thinking fighter is already in its victims.
    pub struck_thinker: bool,
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
        owner: None,
        hitboxes: [None; 4],
        motion: 0,
        life_timer: 0.0,
        bomb_fuse: -1.0,
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
    /// stage_info.cam_info: the camera bounds (Camera_8003118C).
    pub camera: &'a melee_cm::StageCamera,
    /// HSD_GObj_Entities->items, in list order.
    pub items: &'a [ItemView],
    /// PlCo's CPU tables (Fighter_804D64FC).
    pub data: &'a crate::desc::CpuData,
    /// gm_8016C75C: the thinking fighter's player's KO total.
    pub player_kills: i32,
    /// gm_8016C5C0 for each player slot: how many players (or teams) score
    /// above it in the standings, ties sharing a rank.
    pub standings: [u8; 6],
    /// p_ftCommonData->horizontal_stick_deadzone.
    pub horizontal_deadzone: f32,
    /// stage_info.x73C (Ground_801C4DA0): the barrel cannon's aim on the
    /// Kongo Jungles, 0 elsewhere.
    pub barrel_aim: f32,
    /// grLib_801C9E60's drift where the scene knows it (Icicle Mountain's
    /// grIceMt_801FA728: the scroll, downward); None leaves the stage kind
    /// to decide.
    pub stage_drift: Option<hsd_types::Vec3>,
    /// Stack words this think's callees leave behind, which retail later
    /// reads uninitialized.
    pub stale: StaleStack,
}

/// ftCo_800B24B8's r1 in a fighter's CPU proc (Fighter_8006ABA0 ->
/// ftCo_800B3900 -> ftCo_800B2AFC, whose r1 is 0x48 above): ftCo_800A4BEC's
/// frame starts 0x60 below it, so its stwu leaves this back chain at
/// r1 - 0x60. Retail, `corpus_v3_fd_fox_cpu9fox4_e89a89d0e_p0` tick 553
/// and `corpus_v3_fd_fox_cpu5fox4_ef89b3e70_p0` tick 430: the CPU's x54.x.
pub const VERSUS_THINK_FRAME: u32 = 0x804E_E980;
/// ftCo_800AE7AC's frame, under the think's.
pub const ESCAPE_FRAME_SIZE: u32 = 0x68;

/// The return addresses ftCo_800A4BEC's calls save in its caller's LR
/// slot (its own r1 + 4, ftCo_800B24B8's r1 - 0x5C): ftCo_IsAlly's and
/// ftCo_800A0F00's. Its other callees (the blast-zone getters,
/// ftLib_8008732C, sqrtf__Ff, HSD_Randf) save none.
pub mod return_address {
    pub const AFTER_IS_ALLY: u32 = 0x800A_4CB4;
    pub const AFTER_CLOAK_TEST: u32 = 0x800A_4D48;
}

/// Words below ftCo_800B24B8's frame that ftCo_800A75DC reads as
/// ftCo_800A4768's unwritten result (its sp18, ftCo_800B24B8's r1 - 0x60
/// and - 0x5C).
#[derive(Debug, Default)]
pub struct StaleStack {
    /// r1 - 0x5C: the last return address ftCo_800A4BEC's calls saved
    /// there, or the distance ftCo_800A5F4C's item loop stored (sp1C);
    /// None until this think wrote it.
    pub frame_lr_slot: core::cell::Cell<Option<u32>>,
    /// r5 as ftCo_800ADE48 leaves it to the behaviour's script writer
    /// (ftCo_800AC5A0 passes its low byte as a stick); None when unknown.
    pub r5: core::cell::Cell<Option<u32>>,
    /// ftCo_800ADE48's caller's r1, when the port knows it.
    pub caller_frame: core::cell::Cell<Option<u32>>,
    /// The Versus think's r1 (ftCo_800B24B8 or ftCo_800AEFB8).
    pub think_frame: core::cell::Cell<Option<u32>>,
    /// What the last mpCheckFloor left in r5 (`Scene::check_floor`).
    pub floor_r5: core::cell::Cell<Option<melee_mp::SweepR5>>,
}

/// seed_ptr's value (the address of `seed`), which HSD_Randf leaves in r5.
pub const SEED_POINTER: u32 = 0x804D_5F90;

impl StaleStack {
    /// r5 after ftCo_800ADE48's floor probes, as the last mpCheckFloor
    /// left it: after a horizontal floor line its own r1 + 0x44 (8004F188;
    /// ADE48's frame is 0x90 and mpCheckFloor's 0xF0 below the caller's
    /// r1), after a sloped one mpLineIntersection's 0 or 1. An unknown
    /// caller or a probe that tested no line leaves r5 unknown.
    /// Retail, `corpus_v3_bf_fox_cpu9fox4_e89a89d0e_p1` tick 252: x = 0x44.
    pub fn after_floor_probe(&self) {
        use melee_mp::SweepR5;
        self.r5.set(match self.floor_r5.get() {
            Some(SweepR5::OwnFrame) => self
                .caller_frame
                .get()
                .map(|frame| frame - 0x90 - 0xF0 + 0x44),
            Some(SweepR5::Flag(above)) => Some(u32::from(above)),
            Some(SweepR5::Unknown) | None => None,
        });
    }
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
        let (hit, r5) = self.map.check_floor_r5(ax, ay, bx, by);
        self.stale.floor_r5.set(Some(r5));
        hit
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
        outside(self.arena, x, y, half_size)
    }
}

/// [`Scene::outside`] for a caller with only the blast zones.
pub fn outside(arena: &Arena, x: f32, y: f32, half_size: [f32; 2]) -> bool {
    let [half_width, half_height] = half_size;
    x < half_width + arena.left
        || x > arena.right - half_width
        || y < half_height + arena.bottom
        || y > arena.top - half_height
}

/// ftCo_800A1AB4 (0x800A1AB4) and its inlined copies: sqrtf of
/// dx*dx + dy*dy, the first square fused (800A1AD8: fmadds).
pub fn distance(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    sqrtf(fmadds(dx, dx, dy * dy))
}
