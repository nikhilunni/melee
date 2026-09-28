//! `mpisland.c`: the map's "islands", each a run of floor (or ceiling)
//! lines chained by `next_id0` (`prev_id0` for ceilings), with the run's
//! end vertices. The CPU reads them to tell which platform a line belongs
//! to and where it ends.
//!
//! The retail lists are singly linked `mp_UnkStruct0` nodes (0x2C bytes)
//! under `mpIsland_80458E88`; nodes are recycled through free lists and
//! compared by address. Here every node lives in [`Islands::nodes`] and
//! the lists are `next` links between node indices, so list order and node
//! identity (the index) follow the C exactly.
use crate::map::{id_range, CollMap};
use melee_types::mp::{line_flag, line_kind, LineSection, NO_ID};

/// `mp_UnkStruct0` (`mp/types.h:33`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Island {
    /// `+0 next`: the next node of whichever list holds this one.
    pub next: Option<usize>,
    /// `+4 x4`: the first line's start vertex (v0; v1 for a ceiling).
    pub first_vertex: u16,
    /// `+6 x6`: the last line's end vertex.
    pub last_vertex: u16,
    /// `+8 x8`: `first_vertex`'s position (the left end of a floor).
    pub left: hsd_types::Vec3,
    /// `+14 x14`: `last_vertex`'s position (the right end).
    pub right: hsd_types::Vec3,
    /// `+20 x20`: bit 1 set while a line is disabled or hidden.
    pub flags: i32,
    /// `+24 x24`: the first line.
    pub first_line: i16,
    /// `+26 x26`: the last line.
    pub last_line: i16,
    /// `+28 x28`: the joint owning the first line.
    pub joint: i16,
}

/// `mpIsland_80458E88`: list heads, tails and free lists.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Islands {
    pub nodes: Vec<Island>,
    /// `+0`: the floor islands.
    pub floors: Option<usize>,
    /// `+4`: the ceiling islands.
    pub ceilings: Option<usize>,
    /// `+8`: the last static floor node (before the joint lists).
    pub floor_tail: Option<usize>,
    /// `+C`: the last static ceiling node.
    pub ceiling_tail: Option<usize>,
    /// `+10`/`+14`: islands rebuilt from joints' dynamic lines.
    pub dynamic_floors: Option<usize>,
    pub dynamic_ceilings: Option<usize>,
    /// `+18`/`+1C`/`+20`: nodes set aside for reuse.
    pub spare_floors: Option<usize>,
    pub spare_ceilings: Option<usize>,
    pub spare_dynamic: Option<usize>,
}

/// `mpIsland_8005A728`'s `u8 visited[0x600]`.
const MAX_LINES: usize = 0x600;

impl Islands {
    fn alloc(&mut self, island: Island) -> usize {
        self.nodes.push(island);
        self.nodes.len() - 1
    }
    /// The nodes of the list starting at `head`, in order.
    pub fn list(&self, head: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(head, |&i| self.nodes[i].next)
    }
}

impl CollMap {
    /// `mpIsland_8005A728` (retail `0x8005A728`, `mpisland.c:54`): build the
    /// floor and ceiling islands from the loaded lines.
    pub(crate) fn build_islands(&mut self) {
        let mut islands = Islands::default();
        let mut visited = [false; MAX_LINES];
        for (section, chain_next, is_kind) in [
            (LineSection::Floor, true, line_kind::FLOOR),
            (LineSection::Ceiling, false, line_kind::CEILING),
        ] {
            let (start, count) = self.data.section(section);
            let mut previous: Option<usize> = None;
            let mut count = i32::from(count);
            let mut line = i32::from(start);
            while count != 0 {
                let node = islands.alloc(Island::default());
                match previous {
                    Some(p) => islands.nodes[p].next = Some(node),
                    None if chain_next => islands.floors = Some(node),
                    None => islands.ceilings = Some(node),
                }
                previous = Some(node);
                let mut end = line;
                // `next = !line_idx`: the walk stops when it cycles to 0/1.
                let mut next = i32::from(line == 0);
                let mut hidden = false;
                while line != next {
                    visited[end as usize] = true;
                    let flags = self.cl(end).flags;
                    if flags & line_flag::ENABLED == 0 || flags & line_flag::HIDDEN != 0 {
                        hidden = true;
                    }
                    let ml = self.ml(end);
                    next = i32::from(if chain_next { ml.next_id0 } else { ml.prev_id0 });
                    if next == NO_ID || u32::from(self.ml(next).hi_flags) & is_kind == 0 {
                        break;
                    }
                    end = next;
                }
                let (first_vertex, last_vertex) = if chain_next {
                    (self.ml(line).v0_idx, self.ml(end).v1_idx)
                } else {
                    (self.ml(line).v1_idx, self.ml(end).v0_idx)
                };
                let position = |vertex: u16| {
                    let pos = self.v(vertex).pos;
                    hsd_types::Vec3::new(pos.x, pos.y, 0.0)
                };
                islands.nodes[node] = Island {
                    next: None,
                    first_vertex,
                    last_vertex,
                    left: position(first_vertex),
                    right: position(last_vertex),
                    flags: if hidden { 2 } else { 0 },
                    first_line: line as i16,
                    last_line: end as i16,
                    joint: self.joint_from_line(line) as i16,
                };
                count -= 1;
                line += 1;
                while count != 0 && visited[line as usize] {
                    line += 1;
                    count -= 1;
                }
            }
            if chain_next {
                islands.floor_tail = previous;
            } else {
                islands.ceiling_tail = previous;
            }
        }
        self.islands = islands;
    }

    /// `mpIsland_8005AB54` (retail `0x8005AB54`): the floor island holding
    /// `line`, or None for an inactive line.
    pub fn island_of_line(&self, line: i32) -> Option<usize> {
        if !self.line_is_active(line) {
            return None;
        }
        for node in self.islands.list(self.islands.floors) {
            let island = self.islands.nodes[node];
            let mut j = i32::from(island.first_line);
            while j != NO_ID {
                if j == line {
                    return Some(node);
                }
                if j == i32::from(island.last_line) {
                    break;
                }
                let next = i32::from(self.ml(j).next_id0);
                if next == NO_ID || self.cl(next).flags & line_kind::FLOOR == 0 {
                    break;
                }
                j = next;
            }
        }
        None
    }

    /// `mpIsland_8005AC14` (retail `0x8005AC14`): the island of the floor
    /// found sweeping from `pos` down by `-dy`... to `pos.y + dy`.
    pub fn island_below(&mut self, pos: hsd_types::Vec3, dy: f32) -> Option<usize> {
        let hit = self.check_floor(
            pos.x,
            pos.y,
            pos.x,
            pos.y + dy,
            0.0,
            NO_ID,
            NO_ID,
            NO_ID,
            None,
        )?;
        self.island_of_line(hit.line_id)
    }

    /// `mpIsland_8005ACE8` (retail `0x8005ACE8`): the island's (left, right)
    /// ends, refreshed from the joint's floor chains: a floor starting at
    /// the first vertex gives the left end, else one ending at the last
    /// vertex gives the right (one test per line, left first).
    pub fn island_ends(&self, node: usize) -> (hsd_types::Vec3, hsd_types::Vec3) {
        let island = self.islands.nodes[node];
        let (mut left, mut right) = (island.left, island.right);
        let joint = &self.data.joints[island.joint as usize];
        let start = i32::from(joint.floor_start);
        for line in start..start + i32::from(joint.floor_count) {
            let ml = self.ml(line);
            if ml.v0_idx == island.first_vertex {
                left = self.floor_get_left(line);
            } else if ml.v1_idx == island.last_vertex {
                right = self.floor_get_right(line);
            }
        }
        (left, right)
    }

    /// An island's data.
    pub fn island(&self, node: usize) -> &Island {
        &self.islands.nodes[node]
    }

    /// `mpIsland_8005B334` (retail `0x8005B334`, `mpisland.c:578`): after
    /// joint `joint_id`'s lines changed state, refresh the islands over its
    /// vertices and rebuild the islands of its dynamic lines.
    pub(crate) fn update_islands(
        &mut self,
        joint_id: i32,
        vtx_start: i16,
        vtx_count: i16,
        enabled: bool,
    ) {
        let mut islands = std::mem::take(&mut self.islands);
        // Cut the static lists off the dynamic ones.
        match islands.floor_tail {
            Some(tail) => islands.nodes[tail].next = None,
            None => islands.floors = None,
        }
        match islands.ceiling_tail {
            Some(tail) => islands.nodes[tail].next = None,
            None => islands.ceilings = None,
        }
        let (start, count) = (i32::from(vtx_start), i32::from(vtx_count));
        let (floors, spare) = self.refresh_static(
            &mut islands.nodes,
            islands.floors,
            islands.spare_floors,
            start,
            count,
            enabled,
        );
        islands.floors = floors;
        islands.spare_floors = spare;
        let (ceilings, spare) = self.refresh_static(
            &mut islands.nodes,
            islands.ceilings,
            islands.spare_ceilings,
            start,
            count,
            enabled,
        );
        islands.ceilings = ceilings;
        islands.spare_ceilings = spare;
        let (list, spare) = self.rebuild_dynamic(
            &mut islands.nodes,
            islands.dynamic_floors,
            islands.spare_dynamic,
            joint_id,
            line_kind::FLOOR,
            enabled,
        );
        islands.dynamic_floors = list;
        islands.spare_dynamic = spare;
        let (list, spare) = self.rebuild_dynamic(
            &mut islands.nodes,
            islands.dynamic_ceilings,
            islands.spare_dynamic,
            joint_id,
            line_kind::CEILING,
            enabled,
        );
        islands.dynamic_ceilings = list;
        islands.spare_dynamic = spare;
        // Re-link each static list to its dynamic one.
        let floor_tail = islands.list(islands.floors).last();
        match floor_tail {
            Some(tail) => islands.nodes[tail].next = islands.dynamic_floors,
            None => islands.floors = islands.dynamic_floors,
        }
        islands.floor_tail = floor_tail;
        let ceiling_tail = islands.list(islands.ceilings).last();
        match ceiling_tail {
            Some(tail) => islands.nodes[tail].next = islands.dynamic_ceilings,
            None => islands.ceilings = islands.dynamic_ceilings,
        }
        islands.ceiling_tail = ceiling_tail;
        self.islands = islands;
    }

    /// `mpIsland_8005AE1C` (retail `0x8005AE1C`): refresh the end positions
    /// of every island touching vertices `start..start + count` and move
    /// islands between the live list and the spare list by `enabled`. Both
    /// lists come back reversed, as the C prepends.
    fn refresh_static(
        &self,
        nodes: &mut [Island],
        live: Option<usize>,
        spare: Option<usize>,
        start: i32,
        count: i32,
        enabled: bool,
    ) -> (Option<usize>, Option<usize>) {
        let end = start + count;
        let mut kept: Option<usize> = None;
        let mut parked: Option<usize> = None;
        let touches = |island: &Island| {
            let (v0, v1) = (
                i32::from(island.first_vertex),
                i32::from(island.last_vertex),
            );
            !(v0 < start && v1 < start) && (end > v0 || end > v1)
        };
        let mut cur = live;
        while let Some(node) = cur {
            let next = nodes[node].next;
            let mut keep = true;
            if touches(&nodes[node]) {
                self.refresh_ends(&mut nodes[node]);
                if !enabled && nodes[node].flags & 2 == 0 {
                    nodes[node].flags |= 2;
                    keep = false;
                }
            }
            if keep {
                nodes[node].next = kept;
                kept = Some(node);
            } else {
                nodes[node].next = parked;
                parked = Some(node);
            }
            cur = next;
        }
        let mut cur = spare;
        while let Some(node) = cur {
            let next = nodes[node].next;
            let mut revive = false;
            if touches(&nodes[node]) {
                self.refresh_ends(&mut nodes[node]);
                if enabled && nodes[node].flags & 2 != 0 {
                    nodes[node].flags &= !2;
                    revive = true;
                }
            }
            if revive {
                nodes[node].next = kept;
                kept = Some(node);
            } else {
                nodes[node].next = parked;
                parked = Some(node);
            }
            cur = next;
        }
        (kept, parked)
    }

    /// `do_assign`: an island's ends at its vertices' current positions.
    fn refresh_ends(&self, island: &mut Island) {
        let left = self.v(island.first_vertex).pos;
        let right = self.v(island.last_vertex).pos;
        island.left = hsd_types::Vec3::new(left.x, left.y, 0.0);
        island.right = hsd_types::Vec3::new(right.x, right.y, 0.0);
    }

    /// `mpIsland_8005B004` (retail `0x8005B004`): drop joint `joint_id`'s
    /// dynamic islands from `list` (to `spare`), then build one island per
    /// chain of its dynamic lines of kind `kind`.
    fn rebuild_dynamic(
        &self,
        nodes: &mut Vec<Island>,
        list: Option<usize>,
        mut spare: Option<usize>,
        joint_id: i32,
        kind: u32,
        enabled: bool,
    ) -> (Option<usize>, Option<usize>) {
        let type_flag = kind | 0x10;
        let mut kept: Option<usize> = None;
        let mut cur = list;
        while let Some(node) = cur {
            let next = nodes[node].next;
            if i32::from(nodes[node].joint) == joint_id {
                nodes[node].next = spare;
                spare = Some(node);
            } else {
                nodes[node].next = kept;
                kept = Some(node);
            }
            cur = next;
        }
        let mut list = kept;
        let mut visited = [false; MAX_LINES];
        let inner = self.inner(joint_id);
        let lines = id_range(inner.dynamic_start, inner.dynamic_count);
        let is_kind = |line: i32| self.cl(line).flags & 0x1F == type_flag;
        for line in lines {
            if visited[line as usize] || !is_kind(line) {
                continue;
            }
            let mut end = line;
            let mut cycled = false;
            loop {
                visited[end as usize] = true;
                let link = i32::from(self.ml(end).prev_id0);
                if link == NO_ID || !is_kind(link) {
                    break;
                }
                end = link;
                if link == line {
                    cycled = true;
                    break;
                }
            }
            let mut first = line;
            if !cycled {
                loop {
                    visited[first as usize] = true;
                    let link = i32::from(self.ml(first).next_id0);
                    if link == NO_ID || !is_kind(link) {
                        break;
                    }
                    first = link;
                    assert!(link != line, "mpisland.c:518: a dynamic chain loops");
                }
            } else {
                // A closed loop: its leftmost start and rightmost end.
                let mut i = end;
                first = end;
                let mut min_x = -f32::MAX;
                let mut max_x = f32::MAX;
                let cycle_start = i;
                loop {
                    let ml = self.ml(i);
                    if max_x > self.v(ml.v0_idx).pos.x {
                        max_x = self.v(ml.v0_idx).pos.x;
                        end = i;
                    }
                    if min_x > self.v(ml.v1_idx).pos.x {
                        min_x = self.v(ml.v1_idx).pos.x;
                        first = i;
                    }
                    i = i32::from(ml.prev_id0);
                    if i == cycle_start {
                        break;
                    }
                }
            }
            let node = match spare {
                Some(node) => {
                    spare = nodes[node].next;
                    node
                }
                None => {
                    nodes.push(Island::default());
                    nodes.len() - 1
                }
            };
            let (first_vertex, last_vertex) = (self.ml(end).v0_idx, self.ml(first).v1_idx);
            let mut island = Island {
                next: list,
                first_vertex,
                last_vertex,
                flags: if enabled { 0 } else { 2 },
                first_line: end as i16,
                last_line: first as i16,
                joint: joint_id as i16,
                ..Island::default()
            };
            self.refresh_ends(&mut island);
            nodes[node] = island;
            list = Some(node);
        }
        (list, spare)
    }
}
