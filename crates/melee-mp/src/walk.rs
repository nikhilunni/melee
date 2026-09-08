//! Line chain walkers and per-line accessors from `mplib.c`:
//! `mpLineGetNext`/`Prev`, the `mpLine{Next,Prev}Non*` family, the
//! `mp*Get{Left,Right,Top,Bottom}` endpoint finders, and the simple
//! `mpLineGet*` queries.

use hsd_types::Vec3;
use melee_types::mp::{line_flag, line_kind, NO_ID};

use crate::geom::{line_normal, sq};
use crate::map::CollMap;

/// Which link of the line record a walk follows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dir {
    Next,
    Prev,
}

impl CollMap {
    /// `mpLineGetNext` (retail `0x8004DB78`, `mplib.c:1023`): the line after
    /// `line_id`. Prefers the alternate `next_id1` link when that line is
    /// enabled, unhidden, and starts within 2 units of our end; otherwise
    /// `next_id0`.
    pub fn line_next(&self, line_id: i32) -> i32 {
        let line = self.ml(line_id);
        let result = i32::from(line.next_id1);
        if result != -1 {
            let flags = self.cl(result).flags;
            if flags & line_flag::ENABLED != 0 && flags & line_flag::HIDDEN == 0 {
                let v1 = self.v(line.v1_idx);
                let v0 = self.v(self.ml(result).v0_idx);
                if f64::from(sq(v1.pos.x - v0.pos.x) + sq(v1.pos.y - v0.pos.y)) < 4.0 {
                    return result;
                }
            }
        }
        i32::from(line.next_id0)
    }

    /// `mpLineGetPrev` (retail `0x8004DC04`, `mplib.c:1044`).
    pub fn line_prev(&self, line_id: i32) -> i32 {
        let line = self.ml(line_id);
        let result = i32::from(line.prev_id1);
        if result != -1 {
            let flags = self.cl(result).flags;
            if flags & line_flag::ENABLED != 0 && flags & line_flag::HIDDEN == 0 {
                let v0 = self.v(line.v0_idx);
                let v1 = self.v(self.ml(result).v1_idx);
                if f64::from(sq(v0.pos.x - v1.pos.x) + sq(v0.pos.y - v1.pos.y)) < 4.0 {
                    return result;
                }
            }
        }
        i32::from(line.prev_id0)
    }

    #[inline]
    pub(crate) fn line_step(&self, line_id: i32, dir: Dir) -> i32 {
        match dir {
            Dir::Next => self.line_next(line_id),
            Dir::Prev => self.line_prev(line_id),
        }
    }

    /// The eight `mpLine{Next,Prev}Non{Floor,Ceiling,LeftWall,RightWall}`
    /// bodies (`mplib.c:3686-3828`): step along the chain while lines still
    /// carry `kind`; return the first that does not, or -1 if the walk
    /// dead-ends or loops back to `line_id`.
    fn walk_non_kind(&self, line_id: i32, kind: u32, dir: Dir) -> i32 {
        self.check_line_id(line_id);
        let mut new_id = self.line_step(line_id, dir);
        while new_id != -1 && new_id != line_id && self.cl(new_id).flags & kind != 0 {
            new_id = self.line_step(new_id, dir);
        }
        // mpLineIterNonResult
        if new_id != -1 && new_id != line_id {
            new_id
        } else {
            -1
        }
    }

    /// `mpLineNextNonFloor` (retail `0x80052534`).
    pub fn line_next_non_floor(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::FLOOR, Dir::Next)
    }

    /// `mpLinePrevNonFloor` (retail `0x80052700`).
    pub fn line_prev_non_floor(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::FLOOR, Dir::Prev)
    }

    /// `mpLinePrevNonCeiling` (retail `0x800528CC`).
    pub fn line_prev_non_ceiling(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::CEILING, Dir::Prev)
    }

    /// `mpLineNextNonCeiling` (retail `0x80052A98`).
    pub fn line_next_non_ceiling(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::CEILING, Dir::Next)
    }

    /// `mpLineNextNonLeftWall` (retail `0x80052C64`).
    pub fn line_next_non_left_wall(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::LEFT_WALL, Dir::Next)
    }

    /// `mpLinePrevNonLeftWall` (retail `0x80052E30`).
    pub fn line_prev_non_left_wall(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::LEFT_WALL, Dir::Prev)
    }

    /// `mpLinePrevNonRightWall` (retail `0x80052FFC`).
    pub fn line_prev_non_right_wall(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::RIGHT_WALL, Dir::Prev)
    }

    /// `mpLineNextNonRightWall` (retail `0x800531C8`).
    pub fn line_next_non_right_wall(&self, line_id: i32) -> i32 {
        self.walk_non_kind(line_id, line_kind::RIGHT_WALL, Dir::Next)
    }

    /// `mpLineWalkNon` (static inline, `mplib.c:3832`): like
    /// [`Self::walk_non_kind`] but following only the `*_id0` links and
    /// without the loop-back guard.
    fn walk_non_kind_id0(&self, line_id: i32, kind: u32, dir: Dir) -> i32 {
        self.check_line_id(line_id);
        let step = |id: i32| -> i32 {
            match dir {
                Dir::Next => i32::from(self.ml(id).next_id0),
                Dir::Prev => i32::from(self.ml(id).prev_id0),
            }
        };
        let mut new_id = step(line_id);
        while new_id != -1 && self.cl(new_id).flags & kind != 0 {
            new_id = step(new_id);
        }
        if new_id != -1 {
            new_id
        } else {
            -1
        }
    }

    /// `mpLib_80053394_Floor` (retail `0x80053394`): next non-floor via id0.
    pub fn line_next_non_floor_id0(&self, line_id: i32) -> i32 {
        self.walk_non_kind_id0(line_id, line_kind::FLOOR, Dir::Next)
    }

    /// `mpLib_80053448_Floor` (retail `0x80053448`): prev non-floor via id0.
    pub fn line_prev_non_floor_id0(&self, line_id: i32) -> i32 {
        self.walk_non_kind_id0(line_id, line_kind::FLOOR, Dir::Prev)
    }

    /// `mpLib_8005389C_Ceiling` (retail `0x8005389C`): prev non-ceiling via
    /// id0.
    pub fn line_prev_non_ceiling_id0(&self, line_id: i32) -> i32 {
        self.walk_non_kind_id0(line_id, line_kind::CEILING, Dir::Prev)
    }

    /// `mpLib_80053950_Ceiling` (retail `0x80053950`): next non-ceiling via
    /// id0.
    pub fn line_next_non_ceiling_id0(&self, line_id: i32) -> i32 {
        self.walk_non_kind_id0(line_id, line_kind::CEILING, Dir::Next)
    }

    /// Body shared by `mpLib_800534FC_Floor`, `mpLib_800536CC_Floor`,
    /// `mpLib_80053A04_Ceiling`, `mpLib_80053BD4_Ceiling`
    /// (`mplib.c:3923-4089`): walk the chain while lines carry `kind`, but
    /// stop (and return the neighbour) as soon as the step taken was the
    /// alternate `*_id1` link. Returns -1 when the walk leaves `kind`.
    fn walk_kind_until_alt_link(&self, mut line_id: i32, kind: u32, dir: Dir) -> i32 {
        self.check_line_id(line_id);
        let mut new_id = self.line_step(line_id, dir);
        while new_id != -1 {
            let alt = match dir {
                Dir::Next => i32::from(self.ml(line_id).next_id1),
                Dir::Prev => i32::from(self.ml(line_id).prev_id1),
            };
            if self.cl(new_id).flags & kind == 0 {
                new_id = -1;
            } else if new_id != alt {
                line_id = new_id;
                new_id = self.line_step(new_id, dir);
                continue;
            }
            break;
        }
        if new_id != -1 {
            new_id
        } else {
            -1
        }
    }

    /// `mpLib_800534FC_Floor` (retail `0x800534FC`).
    pub fn floor_next_across_joint(&self, line_id: i32) -> i32 {
        self.walk_kind_until_alt_link(line_id, line_kind::FLOOR, Dir::Next)
    }

    /// `mpLib_800536CC_Floor` (retail `0x800536CC`).
    pub fn floor_prev_across_joint(&self, line_id: i32) -> i32 {
        self.walk_kind_until_alt_link(line_id, line_kind::FLOOR, Dir::Prev)
    }

    /// `mpLib_80053A04_Ceiling` (retail `0x80053A04`).
    pub fn ceiling_prev_across_joint(&self, line_id: i32) -> i32 {
        self.walk_kind_until_alt_link(line_id, line_kind::CEILING, Dir::Prev)
    }

    /// `mpLib_80053BD4_Ceiling` (retail `0x80053BD4`).
    pub fn ceiling_next_across_joint(&self, line_id: i32) -> i32 {
        self.walk_kind_until_alt_link(line_id, line_kind::CEILING, Dir::Next)
    }

    /// `mpLib_80053DA4_Floor` (retail `0x80053DA4`, `mplib.c:4091`): follow
    /// `next_id0` while the next line is a floor; return the last floor's
    /// `v1`.
    pub fn floor_chain_right_end_id0(&self, mut line_id: i32) -> Vec3 {
        self.check_line_id(line_id);
        loop {
            let next_id = i32::from(self.ml(line_id).next_id0);
            if next_id != -1 && self.cl(next_id).flags & line_kind::FLOOR != 0 {
                line_id = next_id;
                continue;
            }
            break;
        }
        self.check_line_id(line_id);
        let v = self.v(self.ml(line_id).v1_idx);
        Vec3::new(v.pos.x, v.pos.y, 0.0)
    }

    /// `mpLib_80053ECC_Floor` (retail `0x80053ECC`, `mplib.c:4115`): follow
    /// `prev_id0` while the previous line is a floor; return the first
    /// floor's `v0`.
    pub fn floor_chain_left_end_id0(&self, mut line_id: i32) -> Vec3 {
        self.check_line_id(line_id);
        loop {
            let prev_id = i32::from(self.ml(line_id).prev_id0);
            if prev_id != -1 && self.cl(prev_id).flags & line_kind::FLOOR != 0 {
                line_id = prev_id;
                continue;
            }
            break;
        }
        self.check_line_id(line_id);
        let v = self.v(self.ml(line_id).v0_idx);
        Vec3::new(v.pos.x, v.pos.y, 0.0)
    }

    /// Body of the eight `mp{Floor,Ceiling,LeftWall,RightWall}Get*`
    /// endpoint finders (`mplib.c:4144-4456`): walk `dir` while the kind
    /// nibble stays equal to ours, then return the far vertex of the last
    /// line reached (`v1` walking next, `v0` walking prev).
    fn chain_end(&self, line_id: i32, dir: Dir) -> Vec3 {
        self.check_line_id(line_id);
        let kind = self.cl(line_id).flags & line_kind::KIND_MASK;
        let mut cur = line_id;
        loop {
            let id = self.line_step(cur, dir);
            if id == -1 {
                break;
            }
            if kind == self.cl(id).flags & line_kind::KIND_MASK {
                cur = id;
                continue;
            }
            break;
        }
        let l = self.ml(cur);
        let v = match dir {
            Dir::Next => self.v(l.v1_idx),
            Dir::Prev => self.v(l.v0_idx),
        };
        Vec3::new(v.pos.x, v.pos.y, 0.0)
    }

    /// `mpFloorGetRight` (retail `0x80053FF4`).
    pub fn floor_get_right(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Next)
    }

    /// `mpFloorGetLeft` (retail `0x80054158`).
    pub fn floor_get_left(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Prev)
    }

    /// `mpCeilingGetRight` (retail `0x800542BC`). Ceilings run right to
    /// left, so the right end is reached walking prev.
    pub fn ceiling_get_right(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Prev)
    }

    /// `mpCeilingGetLeft` (retail `0x80054420`).
    pub fn ceiling_get_left(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Next)
    }

    /// `mpLeftWallGetTop` (retail `0x80054584`). Left walls run bottom to
    /// top.
    pub fn left_wall_get_top(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Next)
    }

    /// `mpLeftWallGetBottom` (retail `0x800546E8`).
    pub fn left_wall_get_bottom(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Prev)
    }

    /// `mpRightWallGetTop` (retail `0x8005484C`). Right walls run top to
    /// bottom.
    pub fn right_wall_get_top(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Prev)
    }

    /// `mpRightWallGetBottom` (retail `0x800549B0`).
    pub fn right_wall_get_bottom(&self, line_id: i32) -> Vec3 {
        self.chain_end(line_id, Dir::Next)
    }

    /// `mpLineGetV1Pos` (retail `0x80054B14`).
    pub fn line_get_v1_pos(&self, line_id: i32) -> Vec3 {
        self.check_line_id(line_id);
        let v = self.v(self.ml(line_id).v1_idx);
        Vec3::new(v.pos.x, v.pos.y, 0.0)
    }

    /// `mpLineGetV0Pos` (retail `0x80054BC0`).
    pub fn line_get_v0_pos(&self, line_id: i32) -> Vec3 {
        self.check_line_id(line_id);
        let v = self.v(self.ml(line_id).v0_idx);
        Vec3::new(v.pos.x, v.pos.y, 0.0)
    }

    /// `mpLineGetKind` (retail `0x80054C6C`): the [`line_kind`] nibble of
    /// the runtime flags.
    pub fn line_get_kind(&self, line_id: i32) -> u32 {
        self.check_line_id(line_id);
        self.cl(line_id).flags & line_kind::KIND_MASK
    }

    /// `mpLineGetFlags` (retail `0x80054CEC`): the line's `lo_flags`
    /// (material, platform, ledge).
    pub fn line_get_flags(&self, line_id: i32) -> u32 {
        self.check_line_id(line_id);
        u32::from(self.ml(line_id).lo_flags)
    }

    /// `mpLib_80054D68` (retail `0x80054D68`, `mplib.c:4492`): replace the
    /// material byte of `lo_flags`. The C ORs the whole `flags` word in
    /// after masking only the low byte out; transcribed as is.
    pub fn line_set_material(&mut self, line_id: i32, flags: u32) {
        self.check_line_id(line_id);
        let old = &mut self.ml_mut(line_id).lo_flags;
        *old = ((u32::from(*old) & !line_flag::MATERIAL_MASK) | flags) as u16;
    }

    /// `mpLineGetNormal` (retail `0x80054DFC`, `mplib.c:4502`).
    pub fn line_get_normal(&self, line_id: i32) -> Vec3 {
        self.check_line_id(line_id);
        let (x0, y0, x1, y1) = self.line_pos(line_id);
        line_normal(x0, y0, x1, y1)
    }

    /// `mpLib_80054ED8` (retail `0x80054ED8`, `mplib.c:4522`): is this a
    /// live line id (not -1, enabled, not hidden)? Halts on an out-of-range
    /// id like the C.
    pub fn line_is_active(&self, line_id: i32) -> bool {
        if line_id == NO_ID {
            return false;
        }
        assert!(
            line_id >= 0 && (line_id as usize) < self.data.lines.len(),
            "mplib.c:4636: not found lineID={line_id}"
        );
        let flags = self.cl(line_id).flags;
        !(flags & line_flag::ENABLED == 0 || flags & line_flag::HIDDEN != 0)
    }

    /// `mpLinesConnected` (retail `0x80054F68`, `mplib.c:4580`): is
    /// `target_id` reachable from `start_id` along either direction of the
    /// chain without the kind nibble changing?
    pub fn lines_connected(&self, start_id: i32, target_id: i32) -> bool {
        self.check_line_id(start_id);
        self.check_line_id(target_id);
        if start_id == target_id {
            return true;
        }
        let kind = self.cl(start_id).flags & line_kind::KIND_MASK;
        let mut line_id = self.line_next(start_id);
        while line_id != -1 && kind == self.cl(line_id).flags & line_kind::KIND_MASK {
            if line_id == target_id {
                return true;
            }
            line_id = self.line_next(line_id);
        }
        let mut line_id = self.line_prev(start_id);
        while line_id != -1 && kind == self.cl(line_id).flags & line_kind::KIND_MASK {
            if line_id == target_id {
                return true;
            }
            line_id = self.line_prev(line_id);
        }
        false
    }
}
