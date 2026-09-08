//! Animation objects: `HSD_AObj` from `src/sysdolphin/baselib/aobj.c` and
//! `aobj.h`.
//!
//! An AObj owns a list of [`FObj`] tracks and a frame counter. Each call to
//! [`AObj::interpret_anim`] advances the counter by `framerate`, handles the
//! loop / end-of-animation cases, and runs every track, which pushes its
//! value through the caller's update callback keyed by the track id.
//!
//! What is *not* ported here: `HSD_AObjLoadDesc`'s `obj_id` resolution
//! (`HSD_IDGetDataFromTable` / `HSD_JObjLoadJoint`, which attaches the spline
//! JObj used by the `HSD_A_J_PATH` track), the `HSD_ForeachAnim` object-graph
//! walk (it needs every HSD object type and belongs with the JObj port), and
//! the allocator plumbing. `end_callback_list` is only ever cleared in Melee
//! (`_HSD_AObjForgetMemory`), so [`AObjEndCallback`] models the two retail
//! counters and the invoke condition, with nothing to invoke.

use crate::fobj::{FObj, FObjDesc, ObjUpdateFunc};

/// `AOBJ_REWINDED` (`aobj.h`): set for one step after a loop rewind.
pub const AOBJ_REWINDED: u32 = 1 << 26;
/// `AOBJ_FIRST_PLAY`: the next step evaluates at the current frame without advancing.
pub const AOBJ_FIRST_PLAY: u32 = 1 << 27;
/// `AOBJ_NO_UPDATE`: tracks are advanced but no values are pushed.
pub const AOBJ_NO_UPDATE: u32 = 1 << 28;
/// `AOBJ_LOOP`: rewind to `rewind_frame` when `end_frame` is reached.
pub const AOBJ_LOOP: u32 = 1 << 29;
/// `AOBJ_NO_ANIM`: stopped; `interpret_anim` does nothing.
pub const AOBJ_NO_ANIM: u32 = 1 << 30;

/// `HSD_AObjDesc` (`aobj.h`): the on-disc animation description that
/// `hsd-archive` will produce. `obj_id` is carried but not resolved here.
#[derive(Debug, Clone, PartialEq)]
pub struct AObjDesc {
    /// Only `AOBJ_LOOP | AOBJ_NO_UPDATE` survive `HSD_AObjSetFlags`.
    pub flags: u32,
    pub end_frame: f32,
    /// The `fobjdesc` linked list, in order.
    pub fobjdesc: Vec<FObjDesc>,
    /// Pointer / id of the object the animation references (spline joint
    /// for `HSD_A_J_PATH`). Not resolved by this crate.
    pub obj_id: u32,
}

/// The two file-scope counters `HSD_AObj_804D762C` (animations that ended
/// this pass) and `HSD_AObj_804D7630` (animations still running), plus the
/// predicate `HSD_AObjInvokeCallBacks` (retail `0x803640B0`) evaluates.
///
/// `HSD_JObjAnimAll` resets these, walks the tree, then invokes the end
/// callbacks once if every animation touched has stopped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AObjEndCallback {
    /// `HSD_AObj_804D762C` (`.sbss:0x804D762C`).
    pub ended: i32,
    /// `HSD_AObj_804D7630` (`.sbss:0x804D7630`).
    pub running: i32,
}

impl AObjEndCallback {
    /// `HSD_AObjInitEndCallBack` (retail `0x803640A0`).
    pub fn init(&mut self) {
        self.ended = 0;
        self.running = 0;
    }

    /// The condition under which `HSD_AObjInvokeCallBacks` runs the
    /// `endcallback_list`: at least one animation ended and none is still
    /// running. Melee never registers a callback, so this is informational.
    pub fn should_invoke(&self) -> bool {
        self.ended != 0 && self.running == 0
    }
}

/// `HSD_AObj` (`aobj.h`) minus `hsd_obj`.
#[derive(Debug, Clone, PartialEq)]
pub struct AObj {
    /// `flags`: the `AOBJ_*` bits.
    pub flags: u32,
    /// `curr_frame`.
    pub curr_frame: f32,
    /// `rewind_frame`: loop target.
    pub rewind_frame: f32,
    /// `end_frame`.
    pub end_frame: f32,
    /// `framerate`: frames advanced per step.
    pub framerate: f32,
    /// `fobj`: the track list.
    pub fobj: Vec<FObj>,
}

impl Default for AObj {
    fn default() -> Self {
        AObj::alloc()
    }
}

impl AObj {
    /// `HSD_AObjAlloc` (retail `0x8036453C`): zeroed, stopped, rate 1.
    pub fn alloc() -> AObj {
        AObj {
            flags: AOBJ_NO_ANIM,
            curr_frame: 0.0,
            rewind_frame: 0.0,
            end_frame: 0.0,
            framerate: 1.0,
            fobj: Vec::new(),
        }
    }

    /// `HSD_AObjLoadDesc` (retail `0x8036439C`) without the `obj_id`
    /// resolution (see the module notes).
    pub fn load_desc(desc: &AObjDesc) -> AObj {
        let mut aobj = AObj::alloc();
        aobj.set_flags(desc.flags);
        aobj.set_rewind_frame(0.0);
        aobj.set_end_frame(desc.end_frame);
        aobj.set_fobj(FObj::load_desc_list(&desc.fobjdesc));
        aobj
    }

    /// `HSD_AObjGetFlags` (retail `0x80364004`).
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// `HSD_AObjSetFlags` (retail `0x8036401C`): only `AOBJ_LOOP` and
    /// `AOBJ_NO_UPDATE` can be set from outside.
    pub fn set_flags(&mut self, flags: u32) {
        let flags = flags & (AOBJ_LOOP | AOBJ_NO_UPDATE);
        self.flags |= flags;
    }

    /// `HSD_AObjClearFlags` (retail `0x80364038`).
    pub fn clear_flags(&mut self, flags: u32) {
        let flags = flags & (AOBJ_LOOP | AOBJ_NO_UPDATE);
        self.flags &= !flags;
    }

    /// `HSD_AObjSetFObj` (retail `0x80364054`): replace the track list.
    pub fn set_fobj(&mut self, fobj: Vec<FObj>) {
        self.fobj = fobj;
    }

    /// `HSD_AObjReqAnim` (retail `0x8036410C`): (re)start at `frame`.
    pub fn req_anim(&mut self, frame: f32) {
        self.curr_frame = frame;

        let flags = self.flags & !AOBJ_NO_ANIM;
        self.flags = flags | AOBJ_FIRST_PLAY;

        FObj::req_anim_all(&mut self.fobj, frame);
    }

    /// `HSD_AObjStopAnim` (retail `0x8036414C`): flush pending `KEY` values
    /// through `func` and stop.
    pub fn stop_anim(&mut self, func: Option<&mut ObjUpdateFunc<'_>>) {
        FObj::stop_anim_all(&mut self.fobj, func, self.framerate);
        self.flags |= AOBJ_NO_ANIM;
    }

    /// `HSD_AObjInterpretAnim` (retail `0x80364190`): one animation step.
    ///
    /// `update_func` receives `(obj_type, value)` for every value the tracks
    /// produce; it is the `HSD_ObjUpdateFunc` with the object pointer
    /// captured. All retail callers pass a non-null function; the
    /// `AOBJ_NO_UPDATE` flag is what suppresses updates. `cb` accumulates
    /// the end-callback counters for this pass.
    pub fn interpret_anim(&mut self, update_func: &mut ObjUpdateFunc<'_>, cb: &mut AObjEndCallback) {
        let mut rate: f32;

        if self.flags & AOBJ_NO_ANIM != 0 {
            return;
        }

        if self.flags & AOBJ_FIRST_PLAY != 0 {
            self.flags &= 0xF7FF_FFFF;
            rate = 0.0;
        } else {
            rate = self.framerate;
            self.curr_frame += self.framerate;
        }

        if (self.flags & AOBJ_LOOP != 0) && self.end_frame <= self.curr_frame {
            if self.rewind_frame < self.end_frame {
                FObj::stop_anim_all(&mut self.fobj, Some(update_func), rate);
                let y: f32 = self.end_frame - self.rewind_frame;
                let x: f32 = self.curr_frame - self.rewind_frame;
                self.curr_frame = gekko_math::msl::fmodf(x, y) + self.rewind_frame;
                FObj::req_anim_all(&mut self.fobj, self.curr_frame);
            } else {
                self.curr_frame = self.end_frame;
            }
            rate = 0.0;
            self.flags |= AOBJ_REWINDED;
        } else {
            self.flags &= 0xFBFF_FFFF;
        }

        if self.flags & AOBJ_NO_UPDATE != 0 {
            FObj::interpret_anim_all(&mut self.fobj, None, rate);
        } else {
            FObj::interpret_anim_all(&mut self.fobj, Some(update_func), rate);
        }

        if (self.flags & AOBJ_LOOP == 0) && (self.end_frame <= self.curr_frame) {
            FObj::stop_anim_all(&mut self.fobj, Some(update_func), self.framerate);
            self.flags |= AOBJ_NO_ANIM;
        }

        if self.flags & AOBJ_NO_ANIM != 0 {
            cb.ended += 1;
        } else {
            cb.running += 1;
        }
    }

    /// `HSD_AObjSetRate` (retail `0x8036530C`).
    pub fn set_rate(&mut self, rate: f32) {
        self.framerate = rate;
    }

    /// `HSD_AObjSetRewindFrame` (retail `0x8036531C`).
    pub fn set_rewind_frame(&mut self, frame: f32) {
        self.rewind_frame = frame;
    }

    /// `HSD_AObjSetEndFrame` (retail `0x8036532C`).
    pub fn set_end_frame(&mut self, frame: f32) {
        self.end_frame = frame;
    }

    /// `HSD_AObjGetCurrFrame` (`aobj.h`, inline).
    pub fn curr_frame(&self) -> f32 {
        self.curr_frame
    }

    /// `HSD_AObjGetEndFrame` (`aobj.h`, inline).
    pub fn end_frame(&self) -> f32 {
        self.end_frame
    }

    /// `HSD_AObjSetCurrentFrame` (retail `0x8036533C`): seek a running
    /// animation. Ignored when stopped.
    pub fn set_current_frame(&mut self, frame: f32) {
        if self.flags & AOBJ_NO_ANIM == 0 {
            self.curr_frame = frame;
            self.flags = (self.flags & 0xBFFF_FFFF) | AOBJ_FIRST_PLAY;
            FObj::req_anim_all(&mut self.fobj, frame);
        }
    }
}
