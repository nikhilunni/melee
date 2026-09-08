//! Sweeps and searches from `mplib.c`: the `mpLib_8004DD90`-family surface
//! probes, `mpCheckFloor`/`Ceiling`/`LeftWall`/`RightWall` and their
//! `*Remap` variants, the wall vertex sweeps, `mpLib_8005199C_Floor`,
//! the ledge search `mpLib_80051BA8_Floor`, `mpCheckMultiple`, the floor walk
//! `mpLib_80056C54`, and `mpGetSpeed`.

use gekko_math::fma::fmadds;
use gekko_math::msl::{fabsf, sqrtf};
use hsd_types::Vec3;
use melee_types::mp::{joint_flag, line_flag, line_kind, LineSection, NO_ID};

use crate::geom::{
    differs_by_more_than_1e4, line_intersection, line_intersection_h, line_intersection_v,
    line_normal, remap_2d, sq,
};
use crate::map::{CollMap, F32_MAX};

/// Result of a segment sweep against the map: the `vec_out`, `line_id_out`,
/// `flags_out`, `normal_out` quartet every `mpCheck*` writes on success.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineHit {
    /// Intersection point on the map line (`z` is always 0).
    pub pos: Vec3,
    /// Id of the line hit.
    pub line_id: i32,
    /// That line's `lo_flags` (material, platform, ledge).
    pub flags: u32,
    /// Unit normal of the line, or the axis normal for a flat line.
    pub normal: Vec3,
}

/// Result of `mpLib_8004DD90_Floor` and its ceiling/wall siblings: the line
/// under/over/beside a point after walking along connected lines of the same
/// kind, the signed distance from the point to it along the probe axis, and
/// the line's flags and normal.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceProbe {
    /// The line the point is over (may differ from the id passed in).
    pub line_id: i32,
    /// `y_out` for floors/ceilings, `x_out` for walls: how far to move the
    /// point to land on the line.
    pub delta: f32,
    /// The line's `lo_flags`.
    pub flags: u32,
    /// The line's unit normal.
    pub normal: Vec3,
}

/// Result of the ledge search `mpLib_80051BA8_Floor`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LedgeHit {
    /// Id of the floor line whose end is the ledge.
    pub line_id: i32,
    /// The ledge vertex, x clamped into the search box (`out_vec`).
    pub pos: Vec3,
}

/// Result of the floor walk `mpLib_80056C54`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloorWalk {
    /// The C return value: the full distance was walked along floors.
    pub reached: bool,
    /// `line_id_out`: the line the walk ended on, or -1 if it is not a
    /// floor.
    pub line_id: i32,
    /// `vec_out`: the end point.
    pub pos: Vec3,
    /// `flags_out` and `normal_out`, written only when `line_id != -1`.
    pub surface: Option<(u32, Vec3)>,
}

/// The four surface families the sweeps are specialised over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Surface {
    Floor,
    Ceiling,
    LeftWall,
    RightWall,
}

impl Surface {
    fn section(self) -> LineSection {
        match self {
            Surface::Floor => LineSection::Floor,
            Surface::Ceiling => LineSection::Ceiling,
            Surface::LeftWall => LineSection::LeftWall,
            Surface::RightWall => LineSection::RightWall,
        }
    }

    fn kind(self) -> u32 {
        match self {
            Surface::Floor => line_kind::FLOOR,
            Surface::Ceiling => line_kind::CEILING,
            Surface::LeftWall => line_kind::LEFT_WALL,
            Surface::RightWall => line_kind::RIGHT_WALL,
        }
    }

    /// The normal used when the line is axis-aligned and the H/V
    /// intersection is taken.
    fn flat_normal(self) -> Vec3 {
        match self {
            Surface::Floor => Vec3::new(0.0, 1.0, 0.0),
            Surface::Ceiling => Vec3::new(0.0, -1.0, 0.0),
            Surface::LeftWall => Vec3::new(-1.0, 0.0, 0.0),
            Surface::RightWall => Vec3::new(1.0, 0.0, 0.0),
        }
    }
}

/// The per-line callback of `mpCheckFloor` (`bool (*)(Fighter_GObj*, int)`
/// with the GObj captured): return `false` to skip a line.
pub type LineFilter<'a, 'b> = Option<&'a mut (dyn FnMut(i32) -> bool + 'b)>;

impl CollMap {
    /// Does the joint pass the `TooFar` / `joint_id_skip` / `joint_id_only`
    /// filter every sweep applies?
    #[inline]
    fn joint_passes(&self, jid: i32, joint_id_skip: i32, joint_id_only: i32) -> bool {
        if self.joints[jid as usize].flags & joint_flag::TOO_FAR != 0 {
            return false;
        }
        !(joint_id_skip == jid || (joint_id_only != NO_ID && joint_id_only != jid))
    }

    /// `mpLib_8004ED5C` (retail `0x8004ED5C`, `mplib.c:1565`): a line's
    /// endpoints, each pushed one unit outward along the line when a
    /// neighbour exists on that side. Returns `(x0, y0, x1, y1)`.
    pub fn line_endpoints_extended(&self, line_id: i32) -> (f32, f32, f32, f32) {
        let mut calculated_distance = false;
        let (mut x0, mut y0, mut x1, mut y1) = self.line_pos(line_id);
        let mut distance = 0.0f32;

        if self.line_prev(line_id) != -1 {
            distance = sqrtf(sq(x0 - x1) + sq(y0 - y1));
            if distance > 0.001 {
                x0 += (x0 - x1) / distance;
                y0 += (y0 - y1) / distance;
            }
            calculated_distance = true;
        }

        if self.line_next(line_id) != -1 {
            if !calculated_distance {
                distance = sqrtf(sq(x0 - x1) + sq(y0 - y1));
            }
            if distance > 0.001 {
                x1 += (x1 - x0) / distance;
                y1 += (y1 - y0) / distance;
            }
        }
        (x0, y0, x1, y1)
    }

    // -----------------------------------------------------------------------
    // Point probes: mpLib_8004DD90_Floor .. mpLib_8004E684_RightWall
    // -----------------------------------------------------------------------

    /// `mpLib_8004DD90_Floor` (retail `0x8004DD90`, `mplib.c:1097`): starting
    /// from floor `line_id`, walk to the connected floor line under `pos.x`
    /// and return how far `pos` is below it (`y_out`, plus 0.0001). `None`
    /// if `pos.x` overhangs the floor chain by more than 0.1.
    pub fn floor_probe(&self, line_id: i32, pos: &Vec3) -> Option<SurfaceProbe> {
        self.check_line_id(line_id);
        let mut line_id = line_id;
        let mut dir = 0;
        let mut x;
        let mut x0;
        let mut x1;
        loop {
            let (lx0, _, lx1, _) = self.line_pos(line_id);
            x0 = lx0;
            x1 = lx1;
            x = pos.x;
            if x < x0 {
                if dir != 1 {
                    let new_id = self.line_prev(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::FLOOR == 0 {
                        if f64::from(x - x0) < -0.1 {
                            return None;
                        }
                        x = x0;
                        break;
                    }
                    line_id = new_id;
                    dir = -1;
                } else {
                    x = x0;
                    break;
                }
            } else if x > x1 {
                if dir != -1 {
                    let new_id = self.line_next(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::FLOOR == 0 {
                        if f64::from(x - x1) > 0.1 {
                            return None;
                        }
                        x = x1;
                        break;
                    }
                    line_id = new_id;
                    // The C does not set `dir = 1` here; transcribed as is.
                } else {
                    x = x1;
                    break;
                }
            } else {
                break;
            }
        }
        let flags = u32::from(self.ml(line_id).lo_flags);
        let (_, y0, _, y1) = self.line_pos(line_id);
        // retail 0x8004E02C/30/34: fmuls + fdivs + fadds, not fused.
        // then `+ 0.0001` in f64 and rounded to f32 on store.
        let f = (y1 - y0) * (x - x0) / (x1 - x0) + y0 - pos.y;
        let delta = (f64::from(f) + 0.0001) as f32;
        let normal = line_normal(x0, y0, x1, y1);
        Some(SurfaceProbe {
            line_id,
            delta,
            flags,
            normal,
        })
    }

    /// `mpLib_8004E090_Ceiling` (retail `0x8004E090`, `mplib.c:1175`).
    /// Ceilings run right to left (`v0` is the right end).
    pub fn ceiling_probe(&self, line_id: i32, pos: &Vec3) -> Option<SurfaceProbe> {
        self.check_line_id(line_id);
        let mut line_id = line_id;
        let mut dir = 0;
        let mut x = pos.x;
        let mut x0;
        let mut x1;
        loop {
            let (lx0, _, lx1, _) = self.line_pos(line_id);
            x0 = lx0;
            x1 = lx1;
            if pos.x < x1 {
                if dir != 1 {
                    let new_id = self.line_next(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::CEILING == 0 {
                        if f64::from(pos.x - x1) < -0.1 {
                            return None;
                        }
                        x = x1;
                        break;
                    }
                    line_id = new_id;
                    dir = -1;
                    continue;
                }
                x = x1;
            } else if pos.x > x0 {
                if dir != -1 {
                    let new_id = self.line_prev(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::CEILING == 0 {
                        if f64::from(pos.x - x0) > 0.1 {
                            return None;
                        }
                        x = x0;
                        break;
                    }
                    line_id = new_id;
                    dir = 1;
                    continue;
                }
                x = x0;
            }
            break;
        }
        let flags = u32::from(self.ml(line_id).lo_flags);
        let (_, y0, _, y1) = self.line_pos(line_id);
        // retail 0x8004E334/38/3C: fmuls + fdivs + fadds, not fused; fsub (double) at 0x8004E344.
        let f = (y1 - y0) * (x - x0) / (x1 - x0) + y0 - pos.y;
        let delta = (f64::from(f) - 0.0001) as f32;
        let normal = line_normal(x0, y0, x1, y1);
        Some(SurfaceProbe {
            line_id,
            delta,
            flags,
            normal,
        })
    }

    /// `mpLib_8004E398_LeftWall` (retail `0x8004E398`, `mplib.c:1251`). Left
    /// walls run bottom to top; `delta` is `x_out`, the x distance from
    /// `pos` to the wall.
    pub fn left_wall_probe(&self, line_id: i32, pos: &Vec3) -> Option<SurfaceProbe> {
        self.check_line_id(line_id);
        let mut line_id = line_id;
        let mut dir = 0;
        let mut y = pos.y;
        let mut y0;
        let mut y1;
        loop {
            let (_, ly0, _, ly1) = self.line_pos(line_id);
            y0 = ly0;
            y1 = ly1;
            if pos.y < y0 {
                if dir != 1 {
                    let new_id = self.line_prev(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::LEFT_WALL == 0 {
                        if f64::from(pos.y - y0) < -0.1 {
                            return None;
                        }
                        y = y0;
                        break;
                    }
                    line_id = new_id;
                    dir = -1;
                    continue;
                }
                // The C leaves `y = pos.y` here; transcribed as is.
            } else if pos.y > y1 && dir != -1 {
                let new_id = self.line_next(line_id);
                if new_id == -1 || self.cl(new_id).flags & line_kind::LEFT_WALL == 0 {
                    if f64::from(pos.y - y1) > 0.1 {
                        return None;
                    }
                    y = y1;
                    break;
                }
                line_id = new_id;
                dir = 1;
                continue;
            }
            // `pos.y > y1` with `dir == -1` also leaves `y = pos.y`.
            break;
        }
        let flags = u32::from(self.ml(line_id).lo_flags);
        let (x0, _, x1, _) = self.line_pos(line_id);
        // retail 0x8004E628/2C/30: fmuls + fdivs + fadds, not fused.
        let delta = x0 + (x1 - x0) * (y - y0) / (y1 - y0) - pos.x;
        let normal = line_normal(x0, y0, x1, y1);
        Some(SurfaceProbe {
            line_id,
            delta,
            flags,
            normal,
        })
    }

    /// `mpLib_8004E684_RightWall` (retail `0x8004E684`, `mplib.c:1323`).
    /// Right walls run top to bottom.
    pub fn right_wall_probe(&self, line_id: i32, pos: &Vec3) -> Option<SurfaceProbe> {
        self.check_line_id(line_id);
        let mut line_id = line_id;
        let mut dir = 0;
        let mut y = pos.y;
        let mut y0;
        let mut y1;
        loop {
            let (_, ly0, _, ly1) = self.line_pos(line_id);
            y0 = ly0;
            y1 = ly1;
            if pos.y > y0 {
                if dir != -1 {
                    let new_id = self.line_prev(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::RIGHT_WALL == 0 {
                        if f64::from(pos.y - y0) > 0.1 {
                            return None;
                        }
                        y = y0;
                        break;
                    }
                    line_id = new_id;
                    dir = 1;
                    continue;
                }
                y = y0;
            } else if pos.y < y1 {
                if dir != 1 {
                    let new_id = self.line_next(line_id);
                    if new_id == -1 || self.cl(new_id).flags & line_kind::RIGHT_WALL == 0 {
                        if f64::from(pos.y - y1) < -0.1 {
                            return None;
                        }
                        y = y1;
                        break;
                    }
                    line_id = new_id;
                    dir = -1;
                    continue;
                }
                y = y1;
            }
            break;
        }
        let flags = u32::from(self.ml(line_id).lo_flags);
        let (x0, _, x1, _) = self.line_pos(line_id);
        // retail 0x8004E920/24/28: fmuls + fdivs + fadds, not fused.
        let delta = x0 + ((x1 - x0) * (y - y0)) / (y1 - y0) - pos.x;
        let normal = line_normal(x0, y0, x1, y1);
        Some(SurfaceProbe {
            line_id,
            delta,
            flags,
            normal,
        })
    }

    // -----------------------------------------------------------------------
    // Segment sweeps: mpCheckFloor .. mpCheckRightWallRemap
    // -----------------------------------------------------------------------

    /// Shared body of the eight `mpCheck{Floor,Ceiling,LeftWall,RightWall}`
    /// and `*Remap` sweeps (`mplib.c:1611-2906`). Sweep the segment
    /// `(ax, ay) -> (bx, by)` against every enabled line of `surf`'s kind in
    /// every listed joint and keep the hit nearest `(ax, ay)`.
    ///
    /// The variants differ only in: which section is scanned; whether the
    /// line endpoints are the extended ones (`mpLib_8004ED5C`) or the raw
    /// vertex positions; the `y_offset` floors add; whether the sweep start
    /// is remapped by the joint's motion (`remap`, with the distance signed
    /// by direction); and the axis test / direction gate / axis normal used
    /// for an axis-aligned line. Every op is in the C's order.
    #[allow(clippy::too_many_arguments)]
    fn sweep(
        &mut self,
        surf: Surface,
        remap: bool,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        y_offset: f32,
        line_id_skip: i32,
        joint_id_skip: i32,
        joint_id_only: i32,
        mut cb: LineFilter<'_, '_>,
    ) -> Option<LineHit> {
        let mut min_dist2 = F32_MAX;
        let mut result: Option<LineHit> = None;
        let old_x = ax;
        let old_y = ay;
        let (mut ax, mut ay) = (ax, ay);

        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check_2(ax, ay, bx, by);
        }

        for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k];
            if !self.joint_passes(jid, joint_id_skip, joint_id_only) {
                continue;
            }
            let joint_flags = self.joints[jid as usize].flags;

            for line_id in self.joint_lines_with_dynamic(jid, surf.section()) {
                if let Some(cb) = cb.as_mut() {
                    if !cb(line_id) {
                        continue;
                    }
                }
                if line_id_skip == line_id {
                    continue;
                }
                let flags = self.cl(line_id).flags;
                if flags & surf.kind() == 0
                    || flags & line_flag::ENABLED == 0
                    || flags & line_flag::EMPTY != 0
                {
                    continue;
                }

                let extended = match (surf, remap) {
                    (Surface::Floor, false) | (Surface::Ceiling, _) => true,
                    (Surface::Floor, true) | (Surface::LeftWall, _) | (Surface::RightWall, _) => {
                        false
                    }
                };
                let (x0, mut y0, x1, mut y1) = if extended {
                    self.line_endpoints_extended(line_id)
                } else {
                    self.line_pos(line_id)
                };
                if surf == Surface::Floor {
                    // mpCheckFloor: `y0 += y_offset`; mpCheckFloorRemap:
                    // `y0 = y_offset + pos.y`. Same rounding either way.
                    y0 += y_offset;
                    y1 += y_offset;
                }

                if remap {
                    if joint_flags & joint_flag::REMAP_MASK != 0 {
                        let ml = self.ml(line_id);
                        let v0 = self.v(ml.v0_idx);
                        let v1 = self.v(ml.v1_idx);
                        let (nx, ny) =
                            remap_2d(v0.x10, v0.x14, v1.x10, v1.x14, x0, y0, x1, y1, old_x, old_y);
                        ax = nx;
                        ay = ny;
                    } else {
                        ax = old_x;
                        ay = old_y;
                    }
                }
                let dx = bx - ax;
                let dy = by - ay;

                let sloped = match surf {
                    Surface::Floor | Surface::Ceiling => differs_by_more_than_1e4(y0, y1),
                    Surface::LeftWall | Surface::RightWall => differs_by_more_than_1e4(x0, x1),
                };
                let candidate: Option<(f32, f32, Vec3)> = if sloped {
                    line_intersection(x0, y0, x1, y1, ax, ay, bx, by)
                        .map(|(ix, iy)| (ix, iy, line_normal(x0, y0, x1, y1)))
                } else {
                    let gate = match surf {
                        Surface::Floor => ay >= by,
                        Surface::Ceiling => ay <= by,
                        Surface::LeftWall => ax <= bx,
                        Surface::RightWall => ax >= bx,
                    };
                    if gate {
                        let hit = match surf {
                            Surface::Floor | Surface::Ceiling => {
                                line_intersection_h(x0, y0, x1, ax, ay, bx, by)
                            }
                            Surface::LeftWall | Surface::RightWall => {
                                line_intersection_v(x0, y0, y1, ax, ay, bx, by)
                            }
                        };
                        hit.map(|(ix, iy)| (ix, iy, surf.flat_normal()))
                    } else {
                        None
                    }
                };

                let Some((int_x, int_y, normal)) = candidate else {
                    continue;
                };
                let dist2 = if remap {
                    let dx2 = sq(int_x - old_x);
                    let dy2 = sq(int_y - old_y);
                    let mut dist2 = dx2 + dy2;
                    // retail 0x8004F688/F77C/FE54/FF54, 0x800507A0/0894/0F8C/1080: fmuls + fmadds.
                    if fmadds(dx, int_x - old_x, dy * (int_y - old_y)) < 0.0 {
                        dist2 = -dist2;
                    }
                    dist2
                } else {
                    sq(int_x - ax) + sq(int_y - ay)
                };
                if min_dist2 > dist2 {
                    min_dist2 = dist2;
                    result = Some(LineHit {
                        pos: Vec3::new(int_x, int_y, 0.0),
                        line_id,
                        flags: u32::from(self.ml(line_id).lo_flags),
                        normal,
                    });
                }
            }
        }

        if !already_checked {
            self.uncheck_bounding();
        }
        result
    }

    /// `mpCheckFloor` (retail `0x8004F008`, `mplib.c:1611`): sweep
    /// `(ax, ay) -> (bx, by)` against floors raised by `y_offset`, skipping
    /// line `line_id_skip`, joint `joint_id_skip`, and (if not -1) every
    /// joint but `joint_id_only`. `cb` may veto individual lines.
    #[allow(clippy::too_many_arguments)]
    pub fn check_floor(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        y_offset: f32,
        line_id_skip: i32,
        joint_id_skip: i32,
        joint_id_only: i32,
        cb: LineFilter<'_, '_>,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::Floor,
            false,
            ax,
            ay,
            bx,
            by,
            y_offset,
            line_id_skip,
            joint_id_skip,
            joint_id_only,
            cb,
        )
    }

    /// `mpCheckFloorRemap` (retail `0x8004F400`, `mplib.c:1756`): as
    /// [`Self::check_floor`], with the sweep start carried along by any
    /// joint that moved this frame.
    #[allow(clippy::too_many_arguments)]
    pub fn check_floor_remap(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        y_offset: f32,
        line_id_skip: i32,
        joint_id_skip: i32,
        joint_id_only: i32,
        cb: LineFilter<'_, '_>,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::Floor,
            true,
            ax,
            ay,
            bx,
            by,
            y_offset,
            line_id_skip,
            joint_id_skip,
            joint_id_only,
            cb,
        )
    }

    /// `mpCheckCeiling` (retail `0x8004F8A4`, `mplib.c:1928`).
    #[allow(clippy::too_many_arguments)]
    pub fn check_ceiling(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::Ceiling,
            false,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    /// `mpCheckCeilingRemap` (retail `0x8004FC2C`, `mplib.c:2065`).
    #[allow(clippy::too_many_arguments)]
    pub fn check_ceiling_remap(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::Ceiling,
            true,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    /// `mpCheckLeftWall` (retail `0x800501CC`, `mplib.c:2288`).
    pub fn check_left_wall(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::LeftWall,
            false,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    /// `mpCheckLeftWallRemap` (retail `0x8005057C`, `mplib.c:2424`).
    pub fn check_left_wall_remap(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::LeftWall,
            true,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    /// `mpCheckRightWall` (retail `0x800509B8`, `mplib.c:2598`).
    pub fn check_right_wall(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::RightWall,
            false,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    /// `mpCheckRightWallRemap` (retail `0x80050D68`, `mplib.c:2733`).
    pub fn check_right_wall_remap(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        self.sweep(
            Surface::RightWall,
            true,
            ax,
            ay,
            bx,
            by,
            0.0,
            NO_ID,
            joint_id_skip,
            joint_id_only,
            None,
        )
    }

    // -----------------------------------------------------------------------
    // Wall vertex sweeps: mpLib_800511A4_RightWall, mpLib_800515A0_LeftWall
    // -----------------------------------------------------------------------

    /// Shared body of `mpLib_800511A4_RightWall` and
    /// `mpLib_800515A0_LeftWall` (`mplib.c:2908-3193`). The ECB edge moved
    /// from segment `a` to segment `b`; for each wall vertex, remap its
    /// previous position from `a` to `b` and test whether the vertex's own
    /// motion crosses `b`. Returns the nearest such wall's id.
    #[allow(clippy::too_many_arguments)]
    fn wall_vertex_sweep(
        &mut self,
        surf: Surface,
        a0x: f32,
        a0y: f32,
        a1x: f32,
        a1y: f32,
        b0x: f32,
        b0y: f32,
        b1x: f32,
        b1y: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<i32> {
        let mut min_dist2 = F32_MAX;
        let mut result: Option<i32> = None;
        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check_3(a0x, a0y, a1x, a1y, b0x, b0y, b1x, b1y);
        }

        for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k];
            if !self.joint_passes(jid, joint_id_skip, joint_id_only) {
                continue;
            }
            for line_id in self.joint_lines_with_dynamic(jid, surf.section()) {
                let flags = self.cl(line_id).flags;
                if !(flags & surf.kind() != 0
                    && flags & line_flag::ENABLED != 0
                    && flags & line_flag::EMPTY == 0)
                {
                    continue;
                }
                let ml = *self.ml(line_id);
                for vidx in [ml.v0_idx, ml.v1_idx] {
                    let vtx = self.v(vidx);
                    let x0 = vtx.pos.x;
                    let y0 = vtx.pos.y;
                    let x1 = vtx.x10;
                    let y1 = vtx.x14;
                    let (x, y) = remap_2d(a0x, a0y, a1x, a1y, b0x, b0y, b1x, b1y, x1, y1);
                    let vdx = x0 - x;
                    let vdy = y0 - y;
                    // retail 0x80051358/5C, 0x8005145C/1758/1858: fmuls + fmadds.
                    if fmadds(vdx, vdx, sq(vdy)) > 0.001 {
                        if let Some((int_x, int_y)) =
                            line_intersection(b0x, b0y, b1x, b1y, x, y, x0, y0)
                        {
                            let mut dist2 = sq(int_x - x1) + sq(int_y - y1);
                            // retail 0x800513B0/14B0/17AC/18AC: fmuls + fmadds; squared distance stays unfused.
                            if fmadds(vdx, int_x - x1, vdy * (int_y - y1)) < 0.0 {
                                dist2 = -dist2;
                            }
                            if min_dist2 > dist2 {
                                min_dist2 = dist2;
                                result = Some(line_id);
                            }
                        }
                    }
                }
            }
        }

        if !already_checked {
            self.uncheck_bounding();
        }
        result
    }

    /// `mpLib_800511A4_RightWall` (retail `0x800511A4`, `mplib.c:2908`).
    #[allow(clippy::too_many_arguments)]
    pub fn right_wall_vertex_sweep(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        cx: f32,
        cy: f32,
        dx: f32,
        dy: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<i32> {
        self.wall_vertex_sweep(
            Surface::RightWall,
            ax,
            ay,
            bx,
            by,
            cx,
            cy,
            dx,
            dy,
            joint_id_skip,
            joint_id_only,
        )
    }

    /// `mpLib_800515A0_LeftWall` (retail `0x800515A0`, `mplib.c:3050`).
    #[allow(clippy::too_many_arguments)]
    pub fn left_wall_vertex_sweep(
        &mut self,
        a0x: f32,
        a0y: f32,
        a1x: f32,
        a1y: f32,
        b0x: f32,
        b0y: f32,
        b1x: f32,
        b1y: f32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<i32> {
        self.wall_vertex_sweep(
            Surface::LeftWall,
            a0x,
            a0y,
            a1x,
            a1y,
            b0x,
            b0y,
            b1x,
            b1y,
            joint_id_skip,
            joint_id_only,
        )
    }

    // -----------------------------------------------------------------------
    // Point searches
    // -----------------------------------------------------------------------

    /// `mpLib_8005199C_Floor` (retail `0x8005199C`, `mplib.c:3195`): the
    /// first enabled floor whose x range contains `pos.x` and which lies at
    /// or below `pos.y`, in joint-list order; -1 if none.
    pub fn floor_below(&mut self, pos: &Vec3, joint_id_skip: i32, joint_id_only: i32) -> i32 {
        let mut line_id = NO_ID;
        let x = pos.x;
        let y = pos.y;
        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check_2(x, y, x, y - 30000.0);
        }

        'joints: for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k];
            if self.joints[jid as usize].flags & joint_flag::TOO_FAR != 0 || joint_id_skip == jid {
                continue;
            }
            if joint_id_only != NO_ID && joint_id_only != jid {
                continue;
            }
            for id in self.joint_lines_with_dynamic(jid, LineSection::Floor) {
                let flags = self.cl(id).flags;
                if flags & line_kind::FLOOR != 0
                    && flags & line_flag::ENABLED != 0
                    && flags & line_flag::EMPTY == 0
                {
                    let (x0, y0, x1, y1) = self.line_pos(id);
                    if x >= x0 && x <= x1 {
                        if y >= y0 && y >= y1 {
                            line_id = id;
                            break 'joints;
                        }
                        if differs_by_more_than_1e4(x1, x0) {
                            let dx = x1 - x0;
                            let dy = y1 - y0;
                            // retail 0x80051B1C: fmadds (after fdivs).
                            if y >= fmadds(dy / dx, x - x0, y0) {
                                line_id = id;
                                break 'joints;
                            }
                        }
                    }
                }
            }
        }

        if !already_checked {
            self.uncheck_bounding();
        }
        line_id
    }

    /// `mpLib_80051BA8_Floor` (retail `0x80051BA8`, `mplib.c:3272`): the
    /// ledge search used by `ftCo_Cliff*`. Among enabled floors flagged
    /// `LINE_FLAG_LEDGE` whose bounding box overlaps the search box, pick
    /// for `dir > 0` the one with the smallest `v0.x` (its left end is the
    /// ledge) and for `dir < 0` the one with the largest `v1.x`. Panics on
    /// `dir == 0` (`HSD_ASSERT(3821, 0)`).
    #[allow(clippy::too_many_arguments)]
    pub fn find_ledge(
        &mut self,
        line_id_skip: i32,
        joint_id_skip: i32,
        joint_id_only: i32,
        dir: i32,
        left: f32,
        bottom: f32,
        right: f32,
        top: f32,
    ) -> Option<LedgeHit> {
        let mut min = if dir > 0 {
            F32_MAX
        } else if dir < 0 {
            -F32_MAX
        } else {
            panic!("mplib.c:3821: mpLib_80051BA8_Floor dir == 0");
        };
        let mut ledge_id = NO_ID;
        let mut out_x = 0.0f32;
        let mut out_y = 0.0f32;

        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check(left, bottom, right, top);
        }

        for k in 0..self.joint_list.len() {
            let jid = self.joint_list[k];
            if self.joints[jid as usize].flags & joint_flag::TOO_FAR != 0 {
                continue;
            }
            if joint_id_skip == jid {
                continue;
            }
            if !(joint_id_only == NO_ID || joint_id_only == jid) {
                continue;
            }
            for new_id in self.joint_lines_with_dynamic(jid, LineSection::Floor) {
                if line_id_skip == new_id {
                    continue;
                }
                let flags = self.cl(new_id).flags;
                if !(flags & line_kind::FLOOR != 0
                    && flags & line_flag::ENABLED != 0
                    && flags & line_flag::EMPTY == 0)
                {
                    continue;
                }
                let inner = self.ml(new_id);
                if u32::from(inner.lo_flags) & line_flag::LEDGE == 0 {
                    continue;
                }
                let (x0, y0, x1, y1) = self.line_pos(new_id);
                let (line_left, line_right) = if x0 > x1 { (x1, x0) } else { (x0, x1) };
                let (line_bottom, line_top) = if y0 > y1 { (y1, y0) } else { (y0, y1) };
                // distance between midpoints of two right/left pairs
                let dist_h = fabsf((line_right + line_left) - (right + left));
                if dist_h < (line_right - line_left) + (right - left) {
                    let dist_v = fabsf((line_top + line_bottom) - (top + bottom));
                    if dist_v < (line_top - line_bottom) + (top - bottom) {
                        // we intersect in both axes
                        if dir > 0 {
                            if min > x0 {
                                min = x0;
                                ledge_id = new_id;
                                out_x = x0;
                                out_y = y0;
                            }
                        } else if dir < 0 && min < x1 {
                            min = x1;
                            ledge_id = new_id;
                            out_x = x1;
                            out_y = y1;
                        }
                    }
                }
            }
        }

        if !already_checked {
            self.uncheck_bounding();
        }

        if ledge_id != NO_ID {
            if out_x > right {
                out_x = right;
            } else if out_x < left {
                out_x = left;
            }
            Some(LedgeHit {
                line_id: ledge_id,
                pos: Vec3::new(out_x, out_y, 0.0),
            })
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------
    // Combined sweeps
    // -----------------------------------------------------------------------

    /// `mpCheckMultiple` (retail `0x80051EC8`, `mplib.c:3416`): sweep
    /// against the surfaces selected by `checks` (bit 0 floor, 1 ceiling,
    /// 2 left wall, 3 right wall; bit 4 selects the `*Remap` variants) and
    /// return the hit nearest `(x0, y0)`.
    #[allow(clippy::too_many_arguments)]
    pub fn check_multiple(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        checks: u32,
        joint_id_skip: i32,
        joint_id_only: i32,
    ) -> Option<LineHit> {
        let mut min_dist2 = F32_MAX;
        let mut best: Option<LineHit> = None;
        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check_2(x0, y0, x1, y1);
        }
        let remap = checks & 0x10 != 0;

        if checks & 1 != 0 {
            let hit = if remap {
                self.check_floor_remap(
                    x0,
                    y0,
                    x1,
                    y1,
                    0.0,
                    NO_ID,
                    joint_id_skip,
                    joint_id_only,
                    None,
                )
            } else {
                self.check_floor(
                    x0,
                    y0,
                    x1,
                    y1,
                    0.0,
                    NO_ID,
                    joint_id_skip,
                    joint_id_only,
                    None,
                )
            };
            if let Some(h) = hit {
                let dx = sq(h.pos.x - x0);
                let dy = sq(h.pos.y - y0);
                // The floor result is taken unconditionally.
                min_dist2 = dx + dy;
                best = Some(h);
            }
        }
        for (bit, surf) in [
            (2u32, Surface::Ceiling),
            (4u32, Surface::LeftWall),
            (8u32, Surface::RightWall),
        ] {
            if checks & bit == 0 {
                continue;
            }
            let hit = match (surf, remap) {
                (Surface::Ceiling, false) => {
                    self.check_ceiling(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::Ceiling, true) => {
                    self.check_ceiling_remap(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::LeftWall, false) => {
                    self.check_left_wall(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::LeftWall, true) => {
                    self.check_left_wall_remap(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::RightWall, false) => {
                    self.check_right_wall(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::RightWall, true) => {
                    self.check_right_wall_remap(x0, y0, x1, y1, joint_id_skip, joint_id_only)
                }
                (Surface::Floor, _) => unreachable!(),
            };
            if let Some(h) = hit {
                let dx = sq(h.pos.x - x0);
                let dy = sq(h.pos.y - y0);
                if min_dist2 > dx + dy {
                    min_dist2 = dx + dy;
                    best = Some(h);
                }
            }
        }

        if !already_checked {
            self.uncheck_bounding();
        }
        if min_dist2 < F32_MAX {
            best
        } else {
            None
        }
    }

    /// `mpCheckAllRemap` (retail `0x800524DC`): `mpCheckMultiple` with
    /// `checks = 0x1F`.
    pub fn check_all_remap(
        &mut self,
        joint_id_skip: i32,
        joint_id_only: i32,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
    ) -> Option<LineHit> {
        self.check_multiple(x0, y0, x1, y1, 0x1F, joint_id_skip, joint_id_only)
    }

    /// `mpCheckAll` (retail `0x80052508`): `mpCheckMultiple` with
    /// `checks = 0xF`.
    pub fn check_all(
        &mut self,
        joint_id_skip: i32,
        joint_id_only: i32,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
    ) -> Option<LineHit> {
        self.check_multiple(x0, y0, x1, y1, 0xF, joint_id_skip, joint_id_only)
    }

    // -----------------------------------------------------------------------
    // Along-the-floor walk and platform speed
    // -----------------------------------------------------------------------

    /// `mpLib_80056C54` (retail `0x80056C54`, `mplib.c:5245`): from `pos`
    /// snapped onto floor `line_id`, walk `distance` units along the floor
    /// chain (right if positive, left if negative), passing over walls as
    /// long as their cumulative length stays within `max_wall_run`, and
    /// stopping at ceilings. `None` when `line_id` is inactive or `pos` is
    /// not over the floor chain (the C's early `return false` with nothing
    /// written).
    ///
    /// The C leaves `total_dist_f27` uninitialised; it is first written on
    /// the first floor line, which the initial `floor_probe` guarantees is
    /// where the walk starts. It is zeroed here.
    ///
    /// The step test is the C's `!(dist > remaining)`, which unlike `<=`
    /// also advances on NaN; kept as written.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn walk_along_floor(
        &mut self,
        line_id: i32,
        pos: &Vec3,
        distance: f32,
        max_wall_run: f32,
    ) -> Option<FloorWalk> {
        let mut line_id = line_id;
        let mut var_f25 = distance;
        let mut result = true;
        if !self.line_is_active(line_id) {
            return None;
        }
        let probe = self.floor_probe(line_id, pos)?;
        let mut sp58 = *pos;
        sp58.y += probe.delta;
        let mut sp4c: Vec3;
        let mut dist_f28: f32;
        let mut total_dist_f27 = 0.0f32;

        if var_f25 > 0.0 {
            loop {
                sp4c = self.line_get_v1_pos(line_id);
                let x_f2 = sq(sp58.x - sp4c.x);
                let y_f0 = sq(sp58.y - sp4c.y);
                dist_f28 = sqrtf(x_f2 + y_f0);
                let flags_r0 = self.line_get_kind(line_id);
                let mut advance = false;
                if flags_r0 & (line_kind::RIGHT_WALL | line_kind::LEFT_WALL) != 0 {
                    total_dist_f27 += dist_f28;
                    if total_dist_f27 > max_wall_run {
                        result = false;
                    } else {
                        advance = true;
                    }
                } else if flags_r0 & line_kind::CEILING != 0 {
                    result = false;
                } else {
                    total_dist_f27 = 0.0;
                    advance = true;
                }
                if advance && !(dist_f28 > var_f25) {
                    let new_id_r5 = self.line_next(line_id);
                    if new_id_r5 == -1 {
                        result = false;
                    } else {
                        var_f25 -= dist_f28;
                        line_id = new_id_r5;
                        sp58 = sp4c;
                        continue;
                    }
                }
                break;
            }
        } else {
            var_f25 = -var_f25;
            loop {
                sp4c = self.line_get_v0_pos(line_id);
                let x_f2 = sq(sp58.x - sp4c.x);
                let y_f0 = sq(sp58.y - sp4c.y);
                dist_f28 = sqrtf(x_f2 + y_f0);
                let flags_r0 = self.line_get_kind(line_id);
                let mut advance = false;
                if flags_r0 & (line_kind::RIGHT_WALL | line_kind::LEFT_WALL) != 0 {
                    total_dist_f27 += dist_f28;
                    if total_dist_f27 > max_wall_run {
                        result = false;
                    } else {
                        advance = true;
                    }
                } else if flags_r0 & line_kind::CEILING != 0 {
                    result = false;
                } else {
                    total_dist_f27 = 0.0;
                    advance = true;
                }
                if advance && !(dist_f28 > var_f25) {
                    let new_id_r5 = self.line_prev(line_id);
                    if new_id_r5 == -1 {
                        result = false;
                    } else {
                        var_f25 -= dist_f28;
                        line_id = new_id_r5;
                        sp58 = sp4c;
                        continue;
                    }
                }
                break;
            }
        }

        if line_id != NO_ID && self.line_get_kind(line_id) & line_kind::FLOOR == 0 {
            line_id = NO_ID;
        }
        let surface = if line_id != NO_ID {
            Some((self.line_get_flags(line_id), self.line_get_normal(line_id)))
        } else {
            None
        };
        let out = if result {
            if f64::from(dist_f28) < 0.0001 {
                sp58
            } else {
                let temp_f2_5 = var_f25 / dist_f28;
                // retail 0x800573A0/B4/C8: fmadds (x/y/z).
                Vec3::new(
                    fmadds(temp_f2_5, sp4c.x - sp58.x, sp58.x),
                    fmadds(temp_f2_5, sp4c.y - sp58.y, sp58.y),
                    fmadds(temp_f2_5, sp4c.z - sp58.z, sp58.z),
                )
            }
        } else {
            sp4c
        };
        Some(FloorWalk {
            reached: result,
            line_id,
            pos: out,
            surface,
        })
    }

    /// `mpGetSpeed` (retail `0x800567C0`, `mplib.c:5124`): how far a point
    /// riding line `line_id` moved this frame, from the line's previous and
    /// current vertex positions. `None` if the line is inactive. The debug
    /// ROM's `|speed| > 10000` report is omitted.
    pub fn line_speed(&self, line_id: i32, pos: &Vec3) -> Option<Vec3> {
        if !self.line_is_active(line_id) {
            return None;
        }
        let ml = self.ml(line_id);
        let v0 = self.v(ml.v0_idx);
        let v1 = self.v(ml.v1_idx);
        let (new_x, new_y) = remap_2d(
            v0.x10, v0.x14, v1.x10, v1.x14, v0.pos.x, v0.pos.y, v1.pos.x, v1.pos.y, pos.x, pos.y,
        );
        Some(Vec3::new(new_x - pos.x, new_y - pos.y, 0.0))
    }
}
