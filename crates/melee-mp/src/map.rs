//! [`CollMap`]: the owned runtime collision map, replacing the file-scope
//! statics of `mplib.c` (`groundCollVtx`, `groundCollLine`,
//! `groundCollJoint`, `jointListStart`/`End`, `didCheckBounding`,
//! `mpLib_80458868`, `mpLib_804D64B4`).
//!
//! This file holds load, the joint list, bounding-box culling, joint
//! enable/hide/transform, and the joint callback slots. Line walkers live in
//! `walk.rs`, sweeps and searches in `query.rs`.

use gekko_math::msl::fabsf;
use hsd_anim::mtx::mtx_mult_vec;
use hsd_types::{Mtx, Vec2, Vec3};
use melee_types::mp::{
    joint_flag, line_flag, line_kind, CollData, CollJoint, CollLine, CollVtx, LineSection,
    MapCollData, MapJoint, MapLine, MpCollisionBox, MpLibGroundEnum, NO_ID,
};
use melee_types::GrKind;

/// `mpLib_JointCollisionCallback` (`forward.h:37`). `user_data` is the
/// `Ground*` the stage registered, kept as an opaque id; `coll_x50` is
/// `CollData::x50` converted to `int` at the call site as the C does.
pub type JointCollisionCallback = fn(
    user_data: u32,
    joint_id: i32,
    coll: &mut CollData,
    coll_x50: i32,
    ground_kind: i32,
    delta_y: f32,
);

/// The `cb_0`/`cb_data_0`/`cb_1`/`cb_data_1` slots of `CollJoint`
/// (`types.h:113-119`).
#[derive(Clone, Copy, Debug, Default)]
pub struct JointCallbacks {
    pub cb_0: Option<JointCollisionCallback>,
    pub cb_data_0: u32,
    pub cb_1: Option<JointCollisionCallback>,
    pub cb_data_1: u32,
}

/// What `mpLib_80055E9C` reads from a joint's `HSD_JObj`: its `JOBJ_HIDDEN`
/// flag and its world matrix after `HSD_JObjSetupMatrix`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JobjState {
    pub hidden: bool,
    pub mtx: Mtx,
}

/// The runtime stage collision map. See the crate docs.
#[derive(Clone, Debug)]
pub struct CollMap {
    /// `mpLib_804D64B4`: the archive data. `lines[]` is mutated at runtime
    /// (`mpPruneEmptyLines`, `mpLib_800581DC`, `mpLib_80054D68`,
    /// `mpJointUpdateDynamics`), so it is owned rather than borrowed.
    pub(crate) data: MapCollData,
    /// `groundCollVtx`
    pub(crate) vtx: Vec<CollVtx>,
    /// `groundCollLine`, parallel to `data.lines`.
    pub(crate) lines: Vec<CollLine>,
    /// `groundCollJoint`, parallel to `data.joints`.
    pub(crate) joints: Vec<CollJoint>,
    /// The callback slots of each `CollJoint`.
    pub(crate) joint_cbs: Vec<JointCallbacks>,
    /// The `jointListStart`..`jointListEnd` linked list, in list order.
    pub(crate) joint_list: Vec<i32>,
    /// `didCheckBounding`
    pub(crate) did_check_bounding: bool,
    /// `mpLib_80458868[2]`
    pub(crate) bounds: [MpCollisionBox; 2],
    /// `mpColl_804D64AC` (defined in `mpcoll.c`, incremented by
    /// `mpLib_80055E9C`).
    pub(crate) coll_804d64ac: i32,
    /// The `mpcoll.c` file-scope scratch state.
    pub(crate) coll: crate::mpcoll::CollScratch,
    /// `grDynamicAttr_801CA284(pos, floor_id)`: the stage's per-frame
    /// material override consulted by `mpCollEnd`. `None` behaves as a stage
    /// with no dynamic attributes (returns 0).
    pub dynamic_attr_hook: Option<crate::mpcoll::DynamicAttrHook>,
}

/// `F32_MAX`
pub(crate) const F32_MAX: f32 = f32::MAX;

/// `start..start+count` as line ids, for a `(start, count)` section pair.
#[inline]
pub(crate) fn id_range(start: i16, count: i16) -> std::ops::Range<i32> {
    let s = i32::from(start);
    s..s + i32::from(count)
}

impl CollMap {
    // -----------------------------------------------------------------------
    // Load
    // -----------------------------------------------------------------------

    /// `mpPruneEmptyLines` (retail `0x8004D184`, `mplib.c:833`): mark every
    /// zero-length line `LINE_FLAG_EMPTY`, splice it out of the chains, and
    /// unlink it. Skipped on Poke Floats (`stage_info.grkind == Gr_Kind_Pura`).
    pub fn prune_empty_lines(coll_data: &mut MapCollData, grkind: GrKind) {
        if grkind == GrKind::Pura {
            return;
        }
        let line_count = coll_data.lines.len();
        for i in 0..line_count {
            let line = coll_data.lines[i];
            let v0 = coll_data.verts[usize::from(line.v0_idx)];
            let v1 = coll_data.verts[usize::from(line.v1_idx)];
            if v0.x != v1.x || v0.y != v1.y {
                continue;
            }
            let i16_i = i as i16;
            for other in coll_data.lines.iter_mut() {
                if other.prev_id0 == i16_i {
                    other.prev_id0 = line.prev_id0;
                }
                if other.next_id0 == i16_i {
                    other.next_id0 = line.next_id0;
                }
                // The C splices the id1 links to the pruned line's *id0*
                // neighbours; transcribed as is.
                if other.prev_id1 == i16_i {
                    other.prev_id1 = line.prev_id0;
                }
                if other.next_id1 == i16_i {
                    other.next_id1 = line.next_id0;
                }
            }
            let line = &mut coll_data.lines[i];
            line.hi_flags |= line_flag::EMPTY as u16;
            line.prev_id0 = -1;
            line.next_id0 = -1;
            line.prev_id1 = -1;
            line.next_id1 = -1;
        }
    }

    /// `mpLibLoad` (retail `0x8004D288`, `mplib.c:878`). `scale` is
    /// `Ground_801C0498()` (the stage's `param->y`, or 1.0); `grkind` is
    /// `stage_info.grkind`. The `mpIsland_8005A728` call at the end is the
    /// deferred island hook.
    ///
    /// Panics if `coll_data` has no joints, where the C would dereference a
    /// null `joint`.
    pub fn load(mut coll_data: MapCollData, scale: f32, grkind: GrKind) -> CollMap {
        assert!(!coll_data.joints.is_empty(), "mpLibLoad: joint_count == 0");
        let f31 = scale;
        let mut bounds = [MpCollisionBox::default(); 2];
        bounds[0].right = F32_MAX;
        bounds[0].top = F32_MAX;
        bounds[0].left = -F32_MAX;
        bounds[0].bottom = -F32_MAX;

        let mut joints = Vec::with_capacity(coll_data.joints.len());
        let mut joint_list = Vec::with_capacity(coll_data.joints.len());
        for (i, inner) in coll_data.joints.iter().enumerate() {
            joints.push(CollJoint {
                flags: joint_flag::ENABLED,
                xc: 0,
                xe: true,
                bounding_min: Vec2::new(f31 * inner.left_bound, f31 * inner.bottom_bound),
                bounding_max: Vec2::new(f31 * inner.right_bound, f31 * inner.top_bound),
            });
            joint_list.push(i as i32);
        }
        let joint_cbs = vec![JointCallbacks::default(); joints.len()];

        Self::prune_empty_lines(&mut coll_data, grkind);

        let mut lines = vec![CollLine::default(); coll_data.lines.len()];
        for s in LineSection::ALL {
            let (start, count) = coll_data.section(s);
            for id in id_range(start, count) {
                let idx = id as usize;
                lines[idx].flags = u32::from(coll_data.lines[idx].hi_flags) | line_flag::ENABLED;
            }
        }

        let mut vtx = Vec::with_capacity(coll_data.verts.len());
        for v in &coll_data.verts {
            let f0 = v.x;
            let f2 = f31 * f0;
            let x0 = f0;
            let f0 = v.y;
            let f1 = f31 * f0;
            vtx.push(CollVtx {
                x0,
                x4: f0,
                pos: Vec2::new(f2, f1),
                x10: f2,
                x14: f1,
            });
            if bounds[0].top < f1 {
                bounds[0].top = f1;
            }
            if bounds[0].bottom > f1 {
                bounds[0].bottom = f1;
            }
            if bounds[0].right < f2 {
                bounds[0].right = f2;
            }
            if bounds[0].left > f2 {
                bounds[0].left = f2;
            }
        }

        let mut map = CollMap {
            data: coll_data,
            vtx,
            lines,
            joints,
            joint_cbs,
            joint_list,
            did_check_bounding: false,
            bounds,
            coll_804d64ac: 0,
            coll: Default::default(),
            dynamic_attr_hook: None,
        };
        // mpIsland_8005A728(): deferred (see `island_update`).
        map.uncheck_bounding();
        map
    }

    // -----------------------------------------------------------------------
    // Accessors (mpLib_8004D164, mpGetGroundCollVtx/Line/Joint)
    // -----------------------------------------------------------------------

    /// `mpLib_8004D164` (retail `0x8004D164`): the loaded archive data.
    pub fn data(&self) -> &MapCollData {
        &self.data
    }

    /// `mpGetGroundCollVtx` (retail `0x8004D16C`).
    pub fn vertices(&self) -> &[CollVtx] {
        &self.vtx
    }

    /// `mpGetGroundCollLine` (retail `0x8004D174`).
    pub fn coll_lines(&self) -> &[CollLine] {
        &self.lines
    }

    /// `mpGetGroundCollJoint` (retail `0x8004D17C`).
    pub fn joints(&self) -> &[CollJoint] {
        &self.joints
    }

    /// The active joint list, `jointListStart` to `jointListEnd`.
    pub fn joint_list(&self) -> &[i32] {
        &self.joint_list
    }

    /// `mpLib_80458868`: `[0]` is the bound of every vertex at load, `[1]`
    /// the bound of every enabled floor (`update_floor_bounds`).
    pub fn bounds(&self) -> &[MpCollisionBox; 2] {
        &self.bounds
    }

    /// `mpColl_804D64AC`, the count of joint transform updates.
    pub fn transform_update_count(&self) -> i32 {
        self.coll_804d64ac
    }

    /// `LINEID_CHECK` (`mplib.c:43`): `HSD_ASSERTREPORT` unless
    /// `0 <= line_id < line_count`.
    #[inline]
    pub(crate) fn check_line_id(&self, line_id: i32) {
        assert!(
            line_id != NO_ID && (line_id as usize) < self.data.lines.len() && line_id >= 0,
            "mplib.c: not found lineID={line_id}"
        );
    }

    #[inline]
    pub(crate) fn ml(&self, line_id: i32) -> &MapLine {
        &self.data.lines[line_id as usize]
    }

    #[inline]
    pub(crate) fn ml_mut(&mut self, line_id: i32) -> &mut MapLine {
        &mut self.data.lines[line_id as usize]
    }

    #[inline]
    pub(crate) fn cl(&self, line_id: i32) -> &CollLine {
        &self.lines[line_id as usize]
    }

    #[inline]
    pub(crate) fn v(&self, vtx_idx: u16) -> &CollVtx {
        &self.vtx[usize::from(vtx_idx)]
    }

    #[inline]
    pub(crate) fn inner(&self, joint_id: i32) -> &MapJoint {
        &self.data.joints[joint_id as usize]
    }

    /// World positions of a line's two vertices, `(x0, y0, x1, y1)`.
    #[inline]
    pub(crate) fn line_pos(&self, line_id: i32) -> (f32, f32, f32, f32) {
        let l = self.ml(line_id);
        let v0 = self.v(l.v0_idx);
        let v1 = self.v(l.v1_idx);
        (v0.pos.x, v0.pos.y, v1.pos.x, v1.pos.y)
    }

    /// The line ids of one joint section, then the joint's dynamic lines:
    /// the `goto block_8` pattern every sweep in `mplib.c` uses.
    pub(crate) fn joint_lines_with_dynamic(
        &self,
        joint_id: i32,
        s: LineSection,
    ) -> impl Iterator<Item = i32> {
        let inner = self.inner(joint_id);
        let (s0, c0) = inner.section(s);
        let (s1, c1) = inner.section(LineSection::Dynamic);
        id_range(s0, c0).chain(id_range(s1, c1))
    }

    // -----------------------------------------------------------------------
    // Bounding (mpCheckedBounding .. mpUncheckBounding)
    // -----------------------------------------------------------------------

    /// `mpCheckedBounding` (retail `0x800588C8`).
    pub fn checked_bounding(&self) -> bool {
        self.did_check_bounding
    }

    /// `mpBoundingCheck` (retail `0x800588D0`, `mplib.c:5990`): mark every
    /// listed joint outside the box `CollJoint_TooFar`, and every disabled
    /// or hidden joint too.
    pub fn bounding_check(&mut self, left: f32, bottom: f32, right: f32, top: f32) {
        for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k] as usize;
            let curr = &mut self.joints[jid];
            if curr.flags & joint_flag::ENABLED != 0 && curr.flags & joint_flag::HIDDEN == 0 {
                if curr.flags & joint_flag::B10 != 0 {
                    curr.flags &= !joint_flag::TOO_FAR;
                } else if left > curr.bounding_max.x
                    || right < curr.bounding_min.x
                    || bottom > curr.bounding_max.y
                    || top < curr.bounding_min.y
                {
                    curr.flags |= joint_flag::TOO_FAR;
                } else {
                    curr.flags &= !joint_flag::TOO_FAR;
                }
            } else {
                curr.flags |= joint_flag::TOO_FAR;
            }
        }
        self.did_check_bounding = true;
    }

    /// `mpBoundingCheck2` (retail `0x80058970`, `mplib.c:6019`): the box
    /// spanned by two points.
    pub fn bounding_check_2(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let (left, right) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (bottom, top) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        self.bounding_check(left, bottom, right, top);
    }

    /// `mpBoundingCheck3` (retail `0x800589D0`, `mplib.c:6043`): the box
    /// spanned by four points.
    #[allow(clippy::too_many_arguments)]
    pub fn bounding_check_3(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    ) {
        let mut left = x1;
        let mut bottom = y1;
        let mut right;
        let mut top;
        if x0 > left {
            right = x0;
        } else {
            right = left;
            left = x0;
        }
        if y0 > bottom {
            top = y0;
        } else {
            top = bottom;
            bottom = y0;
        }
        if right < x2 {
            right = x2;
        } else if left > x2 {
            left = x2;
        }
        if top < y2 {
            top = y2;
        } else if bottom > y2 {
            bottom = y2;
        }
        if right < x3 {
            right = x3;
        } else if left > x3 {
            left = x3;
        }
        if top < y3 {
            top = y3;
        } else if bottom > y3 {
            bottom = y3;
        }
        self.bounding_check(left, bottom, right, top);
    }

    /// `mpUncheckBounding` (retail `0x80058AA0`, `mplib.c:6088`).
    pub fn uncheck_bounding(&mut self) {
        for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k] as usize;
            self.joints[jid].flags &= !joint_flag::TOO_FAR;
        }
        self.did_check_bounding = false;
    }

    // -----------------------------------------------------------------------
    // Joint hide / dynamics / transform
    // -----------------------------------------------------------------------

    /// `mpIsland_8005B334(joint_id, vtx_start, vtx_count, enabled)`: the
    /// `mpisland.c` update every joint state change ends with. **Deferred**:
    /// island data only feeds the CPU player (`ftcpuattack.c`) and Link's
    /// hookshot, neither of which is ported. Recorded here so every call
    /// site is visible.
    #[allow(unused_variables)]
    pub(crate) fn island_update(
        &mut self,
        joint_id: i32,
        vtx_start: i16,
        vtx_count: i16,
        enabled: bool,
    ) {
        // TODO(mpisland): port mpisland.c and call mpIsland_8005B334 here.
    }

    /// The `var_r6` computation repeated before most island calls:
    /// enabled, not hidden, not B11.
    #[inline]
    fn island_enabled(flags: u32) -> bool {
        flags & (joint_flag::HIDDEN | joint_flag::B11) == 0 && flags & joint_flag::ENABLED != 0
    }

    fn set_line_flag_over_sections(&mut self, joint_id: i32, flag: u32, set: bool) {
        // The C walks floor, ceiling, left wall, right wall, dynamic in that
        // order; the order is unobservable, but keep it.
        for s in [
            LineSection::Floor,
            LineSection::Ceiling,
            LineSection::LeftWall,
            LineSection::RightWall,
            LineSection::Dynamic,
        ] {
            let (start, count) = self.inner(joint_id).section(s);
            for id in id_range(start, count) {
                if set {
                    self.lines[id as usize].flags |= flag;
                } else {
                    self.lines[id as usize].flags &= !flag;
                }
            }
        }
    }

    /// `mpJointHide` (retail `0x8005541C`, `mplib.c:4688`).
    pub fn joint_hide(&mut self, joint_id: i32) {
        self.joints[joint_id as usize].flags |= joint_flag::HIDDEN;
        self.set_line_flag_over_sections(joint_id, line_flag::HIDDEN, true);
    }

    /// `mpJointUnhide` (retail `0x800557D0`, `mplib.c:4733`). Also resets
    /// every vertex's previous position to its current one.
    pub fn joint_unhide(&mut self, joint_id: i32) {
        self.joints[joint_id as usize].flags &= !joint_flag::HIDDEN;
        self.set_line_flag_over_sections(joint_id, line_flag::HIDDEN, false);
        let inner = *self.inner(joint_id);
        for vid in id_range(inner.vtx_start, inner.vtx_count) {
            let vtx = &mut self.vtx[vid as usize];
            vtx.x10 = vtx.pos.x;
            vtx.x14 = vtx.pos.y;
        }
    }

    /// `mpJointUpdateDynamics` (retail `0x80055C5C`, `mplib.c:4786`):
    /// reclassify each dynamic line of the joint as floor, ceiling, or wall
    /// from its current slope.
    ///
    /// Panics on a zero-length dynamic line (`HSD_ASSERT(4884, 0)`).
    pub fn joint_update_dynamics(&mut self, joint_id: i32) {
        const TAN30: f64 = 0.577350295784245;
        const TAN60: f64 = 1.7320508368950045;
        let joint_flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        for id in id_range(inner.dynamic_start, inner.dynamic_count) {
            let (x0, y0, x1, y1) = self.line_pos(id);
            let dx = x1 - x0;
            let dy = y1 - y0;
            let kind = if dx > 0.0 {
                if f64::from(dy / dx) > TAN60 {
                    line_kind::LEFT_WALL
                } else if f64::from(dy / dx) < -TAN60 {
                    line_kind::RIGHT_WALL
                } else {
                    line_kind::FLOOR
                }
            } else if dx < 0.0 {
                if f64::from(dy / dx) > TAN30 {
                    line_kind::RIGHT_WALL
                } else if f64::from(dy / dx) < -TAN30 {
                    line_kind::LEFT_WALL
                } else {
                    line_kind::CEILING
                }
            } else if dy > 0.0 {
                line_kind::LEFT_WALL
            } else if dy < 0.0 {
                line_kind::RIGHT_WALL
            } else {
                panic!("mplib.c:4884: zero-length dynamic line {id}");
            };
            let line = &mut self.lines[id as usize];
            line.flags = (line.flags & !line_kind::KIND_MASK) | kind;
            let lo_flags = self.data.lines[id as usize].lo_flags;
            if joint_flags & joint_flag::ENABLED != 0
                && u32::from(lo_flags) & line_flag::DYNAMIC_PLATFORM != 0
            {
                if kind & line_kind::FLOOR != 0 {
                    line.flags |= line_flag::ENABLED | line_flag::PLATFORM;
                    self.data.lines[id as usize].lo_flags |= line_flag::PLATFORM as u16;
                } else {
                    line.flags &= !line_flag::ENABLED;
                }
            }
        }
    }

    /// `mpLib_80055E24` (retail `0x80055E24`, `mplib.c:4839`): update
    /// dynamics, then the island hook.
    pub fn joint_update_dynamics_and_island(&mut self, joint_id: i32) {
        self.joint_update_dynamics(joint_id);
        let flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            Self::island_enabled(flags),
        );
    }

    /// `mpLib_80055E9C` (retail `0x80055E9C`, `mplib.c:4855`): per-frame
    /// joint update from its JObj. The C reads `joint->x20` (the JObj set by
    /// `mpLib_800552B0`); here the caller passes the JObj's hidden flag and
    /// set-up matrix, or `None` for a joint with no JObj.
    ///
    /// The static `mpLib_804D64CC` tested at `mplib.c:4943` is never written
    /// anywhere in the game, so the early exit it guards is dead and is not
    /// transcribed.
    pub fn update_joint_transform(&mut self, joint_id: i32, jobj: Option<JobjState>) {
        self.coll_804d64ac += 1;
        let inner = *self.inner(joint_id);
        let vtx_count = inner.vtx_count;
        for vid in id_range(inner.vtx_start, vtx_count) {
            let v = &mut self.vtx[vid as usize];
            v.x10 = v.pos.x;
            v.x14 = v.pos.y;
        }
        let Some(jobj) = jobj else {
            return;
        };

        if jobj.hidden {
            if self.joints[joint_id as usize].flags & joint_flag::HIDDEN == 0 {
                self.joint_hide(joint_id);
                let flags = self.joints[joint_id as usize].flags;
                self.island_update(
                    joint_id,
                    inner.vtx_start,
                    inner.vtx_count,
                    Self::island_enabled(flags),
                );
            }
            return;
        }

        self.joints[joint_id as usize].xe = true;
        let mtx = &jobj.mtx.0;
        let m0_0 = mtx[0][0];
        if m0_0 == mtx[1][1] && m0_0 == mtx[2][2] {
            // Uniform scale, no rotation: apply directly.
            let m0_3 = mtx[0][3];
            let m1_3 = mtx[1][3];
            for vid in id_range(inner.vtx_start, vtx_count) {
                let v = &mut self.vtx[vid as usize];
                // FUSION AUDIT PENDING: v->x0 * m0_0 + m0_3
                v.pos.x = v.x0 * m0_0 + m0_3;
                // FUSION AUDIT PENDING
                v.pos.y = v.x4 * m0_0 + m1_3;
            }
            let joint = &mut self.joints[joint_id as usize];
            // FUSION AUDIT PENDING: each bound is `b * m + t` then +-30.
            joint.bounding_min.x = (inner.left_bound * m0_0 + m0_3) - 30.0;
            joint.bounding_min.y = (inner.bottom_bound * m0_0 + m1_3) - 30.0;
            joint.bounding_max.x = 30.0 + (inner.right_bound * m0_0 + m0_3);
            joint.bounding_max.y = 30.0 + (inner.top_bound * m0_0 + m1_3);
            joint.flags |= joint_flag::B8;
        } else {
            let mut sp28 = Vec3::ZERO;
            for vid in id_range(inner.vtx_start, vtx_count) {
                let v = &mut self.vtx[vid as usize];
                let src = Vec3::new(v.x0, v.x4, 0.0);
                mtx_mult_vec(&jobj.mtx, &src, &mut sp28);
                v.pos.x = sp28.x;
                v.pos.y = sp28.y;
            }
            let f1 = 0.0f32;
            let joint = &mut self.joints[joint_id as usize];
            if f1 != mtx[0][1]
                || f1 != mtx[0][2]
                || f1 != mtx[1][0]
                || f1 != mtx[1][2]
                || f1 != mtx[2][0]
                || f1 != mtx[2][1]
            {
                joint.flags |= joint_flag::B9;
            }
            joint.flags |= joint_flag::B8;
            if joint.flags & joint_flag::B10 == 0 {
                let mut out = Vec3::ZERO;
                mtx_mult_vec(
                    &jobj.mtx,
                    &Vec3::new(inner.left_bound, inner.bottom_bound, 0.0),
                    &mut out,
                );
                joint.bounding_min.x = out.x;
                joint.bounding_min.y = out.y;
                mtx_mult_vec(
                    &jobj.mtx,
                    &Vec3::new(inner.right_bound, inner.top_bound, 0.0),
                    &mut out,
                );
                joint.bounding_max.x = out.x;
                joint.bounding_max.y = out.y;
                if joint.flags & joint_flag::B9 != 0 {
                    mtx_mult_vec(
                        &jobj.mtx,
                        &Vec3::new(inner.right_bound, inner.bottom_bound, 0.0),
                        &mut out,
                    );
                    let f30 = out.x;
                    let f31 = out.y;
                    mtx_mult_vec(
                        &jobj.mtx,
                        &Vec3::new(inner.left_bound, inner.top_bound, 0.0),
                        &mut out,
                    );
                    let f1 = joint.bounding_min.x;
                    let f0 = joint.bounding_max.x;
                    let f2 = out.x;
                    let f3 = out.y;
                    if f1 > f0 {
                        joint.bounding_min.x = f0;
                    }
                    if joint.bounding_min.x > f30 {
                        joint.bounding_min.x = f30;
                    }
                    if joint.bounding_min.x > f2 {
                        joint.bounding_min.x = f2;
                    }
                    if joint.bounding_max.x < f1 {
                        joint.bounding_max.x = f1;
                    }
                    if joint.bounding_max.x < f30 {
                        joint.bounding_max.x = f30;
                    }
                    if joint.bounding_max.x < f2 {
                        joint.bounding_max.x = f2;
                    }
                    let f1 = joint.bounding_min.y;
                    let f0 = joint.bounding_max.y;
                    if f1 > f0 {
                        joint.bounding_min.y = f0;
                    }
                    if joint.bounding_min.y > f31 {
                        joint.bounding_min.y = f31;
                    }
                    if joint.bounding_min.y > f3 {
                        joint.bounding_min.y = f3;
                    }
                    if joint.bounding_max.y < f1 {
                        joint.bounding_max.y = f1;
                    }
                    if joint.bounding_max.y < f31 {
                        joint.bounding_max.y = f31;
                    }
                    if joint.bounding_max.y < f3 {
                        joint.bounding_max.y = f3;
                    }
                }
                joint.bounding_min.x -= 30.0;
                joint.bounding_max.x += 30.0;
                joint.bounding_min.y -= 30.0;
                joint.bounding_max.y += 30.0;
            }
        }

        // after0:
        self.joint_update_dynamics(joint_id);
        // after1:
        if self.joints[joint_id as usize].flags & joint_flag::HIDDEN != 0 {
            self.joint_unhide(joint_id);
        }
        let flags = self.joints[joint_id as usize].flags;
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            Self::island_enabled(flags),
        );
    }

    /// `mpJointUpdateBounding` (retail `0x800565DC`, `mplib.c:5052`): grow
    /// the joint's box to cover every vertex with a 30-unit margin.
    pub fn joint_update_bounding(&mut self, joint_id: i32) {
        let inner = *self.inner(joint_id);
        for vid in id_range(inner.vtx_start, inner.vtx_count) {
            let pos = self.vtx[vid as usize].pos;
            let joint = &mut self.joints[joint_id as usize];
            if joint.bounding_min.x > pos.x - 30.0 {
                joint.bounding_min.x = pos.x - 30.0;
            }
            if joint.bounding_max.x < pos.x + 30.0 {
                joint.bounding_max.x = pos.x + 30.0;
            }
            if joint.bounding_min.y > pos.y - 30.0 {
                joint.bounding_min.y = pos.y - 30.0;
            }
            if joint.bounding_max.y < pos.y + 30.0 {
                joint.bounding_max.y = pos.y + 30.0;
            }
        }
    }

    /// `mpLib_8005667C` (retail `0x8005667C`, `mplib.c:5076`): island hook
    /// only.
    pub fn joint_refresh_island(&mut self, joint_id: i32) {
        let flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            Self::island_enabled(flags),
        );
    }

    // -----------------------------------------------------------------------
    // Vertex / line position writes
    // -----------------------------------------------------------------------

    /// `mpVtxGetPos` (retail `0x800566D8`).
    pub fn vtx_get_pos(&self, vtx_id: i32) -> (f32, f32) {
        let v = &self.vtx[vtx_id as usize];
        (v.pos.x, v.pos.y)
    }

    /// `mpVtxSetPos` (retail `0x800566F8`).
    pub fn vtx_set_pos(&mut self, vtx_id: i32, x: f32, y: f32) {
        let v = &mut self.vtx[vtx_id as usize];
        v.pos.x = x;
        v.pos.y = y;
    }

    /// `mpLineSetPos` (retail `0x80056710`).
    pub fn line_set_pos(&mut self, line_id: i32, x0: f32, y0: f32, x1: f32, y1: f32) {
        let l = *self.ml(line_id);
        self.vtx_set_pos(i32::from(l.v0_idx), x0, y0);
        self.vtx_set_pos(i32::from(l.v1_idx), x1, y1);
    }

    /// `mpLib_80056758` (retail `0x80056758`, `mplib.c:5111`): offset the
    /// line's vertices from their archive positions.
    pub fn line_set_offset(&mut self, line_id: i32, x0: f32, y0: f32, x1: f32, y1: f32) {
        let l = *self.ml(line_id);
        let v = &mut self.vtx[usize::from(l.v0_idx)];
        v.pos.x = v.x0 + x0;
        v.pos.y = v.x4 + y0;
        let v = &mut self.vtx[usize::from(l.v1_idx)];
        v.pos.x = v.x0 + x1;
        v.pos.y = v.x4 + y1;
    }

    // -----------------------------------------------------------------------
    // Joint list (mpJointFromLine .. mpLib_80058044)
    // -----------------------------------------------------------------------

    /// `mpJointFromLine` (retail `0x80056B6C`, `mplib.c:5221`): the joint
    /// whose vertex range contains the line's `v0`, or -1.
    pub fn joint_from_line(&self, line_id: i32) -> i32 {
        if line_id != NO_ID {
            self.check_line_id(line_id);
            let v0_idx = i32::from(self.ml(line_id).v0_idx);
            for (i, inner) in self.data.joints.iter().enumerate() {
                let start = i32::from(inner.vtx_start);
                if start <= v0_idx && v0_idx < start + i32::from(inner.vtx_count) {
                    return i as i32;
                }
            }
        }
        NO_ID
    }

    /// `mpLib_80057424` (retail `0x80057424`, `mplib.c:5373`): snapshot the
    /// joint's vertex positions as the previous positions.
    pub fn joint_snapshot_prev_pos(&mut self, joint_id: i32) {
        let inner = *self.inner(joint_id);
        for vid in id_range(inner.vtx_start, inner.vtx_count) {
            let v = &mut self.vtx[vid as usize];
            v.x10 = v.pos.x;
            v.x14 = v.pos.y;
        }
    }

    /// `mpLib_80057528` (retail `0x80057528`, `mplib.c:5387`): enable one
    /// line.
    pub fn line_enable(&mut self, line_id: i32) {
        let joint_id = self.joint_from_line(line_id);
        if joint_id != NO_ID {
            self.lines[line_id as usize].flags |= line_flag::ENABLED;
            let flags = self.joints[joint_id as usize].flags;
            let inner = *self.inner(joint_id);
            self.island_update(
                joint_id,
                inner.vtx_start,
                inner.vtx_count,
                flags & joint_flag::B11 == 0,
            );
            self.joints[joint_id as usize].xe = true;
        }
    }

    /// `mpLib_800575B0` (retail `0x800575B0`, `mplib.c:5401`): disable one
    /// line.
    pub fn line_disable(&mut self, line_id: i32) {
        let joint_id = self.joint_from_line(line_id);
        if joint_id != NO_ID {
            self.lines[line_id as usize].flags &= !line_flag::ENABLED;
            let flags = self.joints[joint_id as usize].flags;
            let inner = *self.inner(joint_id);
            self.island_update(
                joint_id,
                inner.vtx_start,
                inner.vtx_count,
                flags & joint_flag::B11 == 0,
            );
            self.joints[joint_id as usize].xe = true;
        }
    }

    /// `mpJointListAdd` (retail `0x80057638`, `mplib.c:5415`): enable a joint
    /// and append it to the active list.
    pub fn joint_list_add(&mut self, joint_id: i32) {
        if self.joints[joint_id as usize].flags & joint_flag::ENABLED != 0 {
            return;
        }
        self.joints[joint_id as usize].flags |= joint_flag::ENABLED;
        self.joint_list.push(joint_id);
        // floor, ceiling, left wall, right wall, dynamic
        self.set_line_flag_over_sections(joint_id, line_flag::ENABLED, true);
        self.joint_snapshot_prev_pos(joint_id);
        let flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            flags & joint_flag::B11 == 0,
        );
        self.joints[joint_id as usize].xe = true;
    }

    /// `mpJointListUnlink` (retail `0x80057B4C`, `mplib.c:5483`): remove a
    /// joint from the active list (no flag changes).
    pub fn joint_list_unlink(&mut self, joint_id: i32) {
        if let Some(pos) = self.joint_list.iter().position(|&j| j == joint_id) {
            self.joint_list.remove(pos);
        }
    }

    /// `mpLib_80057BC0` (retail `0x80057BC0`, `mplib.c:5508`): disable a
    /// joint and unlink it.
    pub fn joint_list_remove(&mut self, joint_id: i32) {
        if self.joints[joint_id as usize].flags & joint_flag::ENABLED == 0 {
            return;
        }
        self.joints[joint_id as usize].flags &= !joint_flag::ENABLED;
        self.joint_list_unlink(joint_id);
        self.set_line_flag_over_sections(joint_id, line_flag::ENABLED, false);
        let inner = *self.inner(joint_id);
        self.island_update(joint_id, inner.vtx_start, inner.vtx_count, false);
        self.joints[joint_id as usize].xe = true;
    }

    /// `mpLib_80057FDC` (retail `0x80057FDC`, `mplib.c:5568`): clear B11.
    pub fn joint_clear_b11(&mut self, joint_id: i32) {
        self.joints[joint_id as usize].flags &= !joint_flag::B11;
        let flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            Self::island_enabled(flags),
        );
    }

    /// `mpLib_80058044` (retail `0x80058044`, `mplib.c:5585`): set B11.
    pub fn joint_set_b11(&mut self, joint_id: i32) {
        self.joints[joint_id as usize].flags |= joint_flag::B11;
        let flags = self.joints[joint_id as usize].flags;
        let inner = *self.inner(joint_id);
        self.island_update(
            joint_id,
            inner.vtx_start,
            inner.vtx_count,
            Self::island_enabled(flags),
        );
    }

    /// `mpJointSetB10` (retail `0x800580AC`).
    pub fn joint_set_b10(&mut self, joint_id: i32) {
        self.joints[joint_id as usize].flags |= joint_flag::B10;
    }

    // -----------------------------------------------------------------------
    // Callbacks
    // -----------------------------------------------------------------------

    /// `mpJointSetCb1` (retail `0x800580C8`).
    pub fn joint_set_cb1(&mut self, joint_id: i32, user_data: u32, cb: JointCollisionCallback) {
        let c = &mut self.joint_cbs[joint_id as usize];
        c.cb_0 = Some(cb);
        c.cb_data_0 = user_data;
    }

    /// `mpJointClearCb1` (retail `0x800580E0`).
    pub fn joint_clear_cb1(&mut self, joint_id: i32) {
        let c = &mut self.joint_cbs[joint_id as usize];
        c.cb_0 = None;
        c.cb_data_0 = 0;
    }

    /// `mpJointGetCb1` (retail `0x800580FC`): `(cb, user_data)`.
    pub fn joint_get_cb1(&self, joint_id: i32) -> (Option<JointCollisionCallback>, u32) {
        let c = &self.joint_cbs[joint_id as usize];
        (c.cb_0, c.cb_data_0)
    }

    /// `mpJointSetCb2` (retail `0x800581A4`).
    pub fn joint_set_cb2(&mut self, joint_id: i32, user_data: u32, cb: JointCollisionCallback) {
        let c = &mut self.joint_cbs[joint_id as usize];
        c.cb_1 = Some(cb);
        c.cb_data_1 = user_data;
    }

    /// `mpJointGetCb2` (retail `0x800581BC`): `(cb, user_data)`.
    pub fn joint_get_cb2(&self, joint_id: i32) -> (Option<JointCollisionCallback>, u32) {
        let c = &self.joint_cbs[joint_id as usize];
        (c.cb_1, c.cb_data_1)
    }

    /// `mpLib_8005811C` (retail `0x8005811C`, `mplib.c:5631`): fire the
    /// ledge's joint `cb_0` with `ground_kind = 3` when a ledge is grabbed.
    /// `coll->x50` is a float passed to an `int` parameter in the C.
    pub fn notify_ledge_grab(&mut self, coll: &mut CollData, ledge_id: i32) {
        if ledge_id != NO_ID {
            let joint_id = self.joint_from_line(ledge_id);
            if joint_id != NO_ID {
                let c = self.joint_cbs[joint_id as usize];
                if let Some(cb) = c.cb_0 {
                    cb(
                        c.cb_data_0,
                        joint_id,
                        coll,
                        coll.x50 as i32,
                        MpLibGroundEnum::LEDGE,
                        0.0,
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Cross-joint chain stitching (mpLib_800581DC, mpLib_80058560)
    // -----------------------------------------------------------------------

    /// `mpLib_800581DC` (retail `0x800581DC`, `mplib.c:5668`): rebuild the
    /// `*_id1` alternate links between two joints. First drop every id1 link
    /// whose partner does not link back, then link every `v0`/`v1` vertex
    /// pair that lies within 2 units.
    ///
    /// The nearby test is the C's `!(ABS(dx) < 2.0)`, which unlike `>= 2.0`
    /// also rejects NaN; kept as written.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn stitch_joints(&mut self, joint_id0: i32, joint_id1: i32) {
        let j0 = *self.inner(joint_id0);
        let j1 = *self.inner(joint_id1);
        for s in LineSection::ALL {
            for inner in [j0, j1] {
                let (start, count) = inner.section(s);
                for idx in id_range(start, count) {
                    let temp = i32::from(self.ml(idx).prev_id1);
                    if temp != -1 {
                        let temp = i32::from(self.ml(temp).next_id1);
                        if temp != -1 && idx != temp {
                            self.ml_mut(idx).prev_id1 = -1;
                        }
                    }
                    let temp = i32::from(self.ml(idx).next_id1);
                    if temp != -1 {
                        let temp = i32::from(self.ml(temp).prev_id1);
                        if temp != -1 && idx != temp {
                            self.ml_mut(idx).next_id1 = -1;
                        }
                    }
                }
            }
        }

        // for every pair of verts
        for vstart0 in id_range(j0.vtx_start, j0.vtx_count) {
            let v0 = self.vtx[vstart0 as usize];
            for vid in id_range(j1.vtx_start, j1.vtx_count) {
                let v1 = self.vtx[vid as usize];
                // ensure they are nearby
                if !(fabsf(v0.pos.x - v1.pos.x) < 2.0) || !(fabsf(v0.pos.y - v1.pos.y) < 2.0) {
                    continue;
                }
                // find every line with the first vert
                for s in LineSection::ALL {
                    let (lstart, lcount) = j0.section(s);
                    for lid in id_range(lstart, lcount) {
                        if vstart0 == i32::from(self.ml(lid).v0_idx) {
                            // if the first vert is that line's v0
                            // find every line with the second vert as v1
                            for s2 in LineSection::ALL {
                                let (ls2, lc2) = j1.section(s2);
                                for lid2 in id_range(ls2, lc2) {
                                    if vid == i32::from(self.ml(lid2).v1_idx) {
                                        self.ml_mut(lid).prev_id1 = lid2 as i16;
                                        self.ml_mut(lid2).next_id1 = lid as i16;
                                    }
                                }
                            }
                        } else if vstart0 == i32::from(self.ml(lid).v1_idx) {
                            // else if the first vert is that line's v1
                            // find every line with the second vert as v0
                            for s2 in LineSection::ALL {
                                let (ls2, lc2) = j1.section(s2);
                                for lid2 in id_range(ls2, lc2) {
                                    if vid == i32::from(self.ml(lid2).v0_idx) {
                                        self.ml_mut(lid).next_id1 = lid2 as i16;
                                        self.ml_mut(lid2).prev_id1 = lid as i16;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// `mpLib_80058560` (retail `0x80058560`, `mplib.c:5831`): stitch every
    /// pair of enabled, unhidden joints.
    pub fn stitch_all_joints(&mut self) {
        let n = self.data.joints.len() as i32;
        for i in 0..n - 1 {
            for j in i + 1..n {
                let fi = self.joints[i as usize].flags;
                let fj = self.joints[j as usize].flags;
                if fi & joint_flag::ENABLED != 0
                    && fi & joint_flag::HIDDEN == 0
                    && fj & joint_flag::ENABLED != 0
                    && fj & joint_flag::HIDDEN == 0
                {
                    self.stitch_joints(i, j);
                }
            }
        }
    }

    /// `mpLib_80058614_Floor` (retail `0x80058614`, `mplib.c:5854`): if any
    /// joint is dirty (`xE`), clear every dirty bit and recompute
    /// `bounds[1]` over every enabled floor line of every enabled, unhidden
    /// joint. This is the per-frame body of the `mpLib_800587FC` GObj proc.
    pub fn update_floor_bounds(&mut self) {
        let count = self.data.joints.len();
        if !self.joints.iter().any(|j| j.xe) {
            return;
        }
        let mut right = -F32_MAX;
        let mut top = -F32_MAX;
        let mut left = F32_MAX;
        let mut bottom = F32_MAX;
        for i in 0..count {
            self.joints[i].xe = false;
            let flags = self.joints[i].flags;
            if flags & joint_flag::ENABLED == 0 || flags & joint_flag::HIDDEN != 0 {
                continue;
            }
            for id in self.joint_lines_with_dynamic(i as i32, LineSection::Floor) {
                let f = self.cl(id).flags;
                if f & line_kind::FLOOR == 0 || f & line_flag::ENABLED == 0 {
                    continue;
                }
                let (x0, y0, x1, y1) = self.line_pos(id);
                if top < y0 {
                    top = y0;
                }
                if bottom > y0 {
                    bottom = y0;
                }
                if right < x0 {
                    right = x0;
                }
                if left > x0 {
                    left = x0;
                }
                if top < y1 {
                    top = y1;
                }
                if bottom > y1 {
                    bottom = y1;
                }
                if right < x1 {
                    right = x1;
                }
                if left > x1 {
                    left = x1;
                }
            }
        }
        self.bounds[1] = MpCollisionBox {
            top,
            bottom,
            left,
            right,
        };
    }

    /// `mpLib_80058820` (retail `0x80058820`, `mplib.c:5970`) minus the GObj
    /// it creates: reset both collision boxes to +-10000.
    pub fn reset_bounds(&mut self) {
        self.bounds[0] = MpCollisionBox {
            right: 10000.0,
            top: 10000.0,
            left: -10000.0,
            bottom: -10000.0,
        };
        self.bounds[1] = self.bounds[0];
    }
}
