//! [`CollMapBuilder`]: construct a [`MapCollData`] by hand, for tests and
//! tooling until `hsd-archive` can read a stage's `coll_data` node.
//!
//! The builder does what the stage exporter did: it sorts lines into the
//! five kind sections (floor, ceiling, right wall, left wall, dynamic),
//! keeps each group's lines and vertices contiguous so a `MapJoint` can
//! describe them as `(start, count)` ranges, links `prev_id0`/`next_id0`
//! wherever one line's `v1` is another's `v0`, and computes each group's
//! bound from its vertices. The `*_id1` links start at -1, as on stages
//! that never call `mpLib_800581DC`.

use hsd_types::Vec2;
use melee_types::mp::{line_kind, LineSection, MapCollData, MapJoint, MapLine};

/// A line as given to the builder, before section sorting.
#[derive(Clone, Copy, Debug)]
struct PendingLine {
    section: LineSection,
    group: u16,
    v0: u16,
    v1: u16,
    lo_flags: u16,
}

/// Builds a [`MapCollData`]. Vertex ids returned by [`Self::vertex`] are
/// stable through [`Self::build`]: the builder allocates vertices in group
/// order, so callers must add a group's vertices before moving on to the
/// next group (a vertex added to an earlier group after a later group has
/// started is an error).
#[derive(Clone, Debug, Default)]
pub struct CollMapBuilder {
    verts: Vec<(u16, Vec2)>,
    lines: Vec<PendingLine>,
    group_count: u16,
}

impl CollMapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a vertex to `group` at `(x, y)` in archive units. Returns its
    /// index.
    ///
    /// Panics if `group` is lower than the group of the last vertex added.
    pub fn vertex(&mut self, group: u16, x: f32, y: f32) -> u16 {
        if let Some(&(last_group, _)) = self.verts.last() {
            assert!(
                group >= last_group,
                "CollMapBuilder: vertices of group {group} must precede group {last_group}"
            );
        }
        self.group_count = self.group_count.max(group + 1);
        self.verts.push((group, Vec2::new(x, y)));
        (self.verts.len() - 1) as u16
    }

    /// Add a line of `section` from vertex `v0` to `v1` in `group`, with the
    /// given `lo_flags` (material byte, `PLATFORM`, `LEDGE`, ...).
    pub fn line(
        &mut self,
        section: LineSection,
        group: u16,
        v0: u16,
        v1: u16,
        lo_flags: u16,
    ) -> &mut Self {
        assert!(
            usize::from(v0) < self.verts.len() && usize::from(v1) < self.verts.len(),
            "CollMapBuilder: line references an unknown vertex"
        );
        self.group_count = self.group_count.max(group + 1);
        self.lines.push(PendingLine {
            section,
            group,
            v0,
            v1,
            lo_flags,
        });
        self
    }

    /// A floor line, `v0` at the left end.
    pub fn floor(&mut self, group: u16, v0: u16, v1: u16, lo_flags: u16) -> &mut Self {
        self.line(LineSection::Floor, group, v0, v1, lo_flags)
    }

    /// A ceiling line, `v0` at the right end.
    pub fn ceiling(&mut self, group: u16, v0: u16, v1: u16, lo_flags: u16) -> &mut Self {
        self.line(LineSection::Ceiling, group, v0, v1, lo_flags)
    }

    /// A right wall (normal pointing +x), `v0` at the top.
    pub fn right_wall(&mut self, group: u16, v0: u16, v1: u16, lo_flags: u16) -> &mut Self {
        self.line(LineSection::RightWall, group, v0, v1, lo_flags)
    }

    /// A left wall (normal pointing -x), `v0` at the bottom.
    pub fn left_wall(&mut self, group: u16, v0: u16, v1: u16, lo_flags: u16) -> &mut Self {
        self.line(LineSection::LeftWall, group, v0, v1, lo_flags)
    }

    /// A dynamic line, reclassified from its slope at runtime.
    pub fn dynamic(&mut self, group: u16, v0: u16, v1: u16, lo_flags: u16) -> &mut Self {
        self.line(LineSection::Dynamic, group, v0, v1, lo_flags)
    }

    /// Produce the archive-shaped data. Lines are laid out section by
    /// section, and within each section group by group, so every
    /// `(start, count)` pair in the result is contiguous.
    pub fn build(&self) -> MapCollData {
        let groups = usize::from(self.group_count);
        // Order lines: section-major, group-minor, insertion order within.
        let mut order: Vec<usize> = (0..self.lines.len()).collect();
        order.sort_by_key(|&i| (self.lines[i].section as usize, self.lines[i].group, i));

        let mut lines: Vec<MapLine> = Vec::with_capacity(order.len());
        for &i in &order {
            let p = self.lines[i];
            let kind = match p.section {
                LineSection::Floor | LineSection::Dynamic => line_kind::FLOOR,
                LineSection::Ceiling => line_kind::CEILING,
                LineSection::RightWall => line_kind::RIGHT_WALL,
                LineSection::LeftWall => line_kind::LEFT_WALL,
            };
            lines.push(MapLine {
                v0_idx: p.v0,
                v1_idx: p.v1,
                prev_id0: -1,
                next_id0: -1,
                prev_id1: -1,
                next_id1: -1,
                hi_flags: kind as u16,
                lo_flags: p.lo_flags,
            });
        }

        // Link chains on shared vertices: A.v1 == B.v0 => A.next_id0 = B.
        for a in 0..lines.len() {
            for b in 0..lines.len() {
                if a != b && lines[a].v1_idx == lines[b].v0_idx {
                    lines[a].next_id0 = b as i16;
                    lines[b].prev_id0 = a as i16;
                }
            }
        }

        // Whole-map and per-group section ranges.
        let mut data = MapCollData {
            verts: self.verts.iter().map(|&(_, v)| v).collect(),
            lines,
            ..Default::default()
        };
        let mut joints = vec![MapJoint::default(); groups];
        for s in LineSection::ALL {
            let sec_ids: Vec<usize> = order
                .iter()
                .enumerate()
                .filter(|(_, &i)| self.lines[i].section == s)
                .map(|(pos, _)| pos)
                .collect();
            let (start, count) = range_of(&sec_ids);
            set_section(&mut data, s, start, count);
            for (g, joint) in joints.iter_mut().enumerate() {
                let ids: Vec<usize> = sec_ids
                    .iter()
                    .copied()
                    .filter(|&pos| usize::from(self.lines[order[pos]].group) == g)
                    .collect();
                let (start, count) = range_of(&ids);
                set_joint_section(joint, s, start, count);
            }
        }

        // Vertex ranges and bounds per group.
        for (g, joint) in joints.iter_mut().enumerate() {
            let ids: Vec<usize> = self
                .verts
                .iter()
                .enumerate()
                .filter(|(_, &(vg, _))| usize::from(vg) == g)
                .map(|(i, _)| i)
                .collect();
            let (start, count) = range_of(&ids);
            joint.vtx_start = start;
            joint.vtx_count = count;
            let mut left = f32::MAX;
            let mut right = -f32::MAX;
            let mut bottom = f32::MAX;
            let mut top = -f32::MAX;
            for &i in &ids {
                let v = self.verts[i].1;
                if v.x < left {
                    left = v.x;
                }
                if v.x > right {
                    right = v.x;
                }
                if v.y < bottom {
                    bottom = v.y;
                }
                if v.y > top {
                    top = v.y;
                }
            }
            if ids.is_empty() {
                left = 0.0;
                right = 0.0;
                bottom = 0.0;
                top = 0.0;
            }
            joint.left_bound = left;
            joint.right_bound = right;
            joint.bottom_bound = bottom;
            joint.top_bound = top;
        }
        data.joints = joints;
        data
    }
}

/// `(start, count)` of a sorted, contiguous list of indices; `(0, 0)` when
/// empty. Panics if the indices are not contiguous, which the builder's
/// ordering guarantees they are.
fn range_of(ids: &[usize]) -> (i16, i16) {
    match ids.first() {
        None => (0, 0),
        Some(&first) => {
            for (k, &id) in ids.iter().enumerate() {
                assert_eq!(id, first + k, "CollMapBuilder: non-contiguous section");
            }
            (first as i16, ids.len() as i16)
        }
    }
}

fn set_section(d: &mut MapCollData, s: LineSection, start: i16, count: i16) {
    match s {
        LineSection::Floor => {
            d.floor_start = start;
            d.floor_count = count;
        }
        LineSection::Ceiling => {
            d.ceiling_start = start;
            d.ceiling_count = count;
        }
        LineSection::RightWall => {
            d.right_wall_start = start;
            d.right_wall_count = count;
        }
        LineSection::LeftWall => {
            d.left_wall_start = start;
            d.left_wall_count = count;
        }
        LineSection::Dynamic => {
            d.dynamic_start = start;
            d.dynamic_count = count;
        }
    }
}

fn set_joint_section(j: &mut MapJoint, s: LineSection, start: i16, count: i16) {
    match s {
        LineSection::Floor => {
            j.floor_start = start;
            j.floor_count = count;
        }
        LineSection::Ceiling => {
            j.ceiling_start = start;
            j.ceiling_count = count;
        }
        LineSection::RightWall => {
            j.right_wall_start = start;
            j.right_wall_count = count;
        }
        LineSection::LeftWall => {
            j.left_wall_start = start;
            j.left_wall_count = count;
        }
        LineSection::Dynamic => {
            j.dynamic_start = start;
            j.dynamic_count = count;
        }
    }
}
