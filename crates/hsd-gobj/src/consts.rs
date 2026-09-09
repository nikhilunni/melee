//! Constants: sentinel values, GObj classifiers, and the `p_link` /
//! `gx_link` / proc-priority numbers Melee hands to the scheduler.
//!
//! Every value cites the decomp line it was read from. The `p_link` and
//! priority numbers are what determine the per-frame ordering the oracle
//! compares (see the crate docs), so keep them literal.

/// `gobj->gx_link` when the object is not on any GX (render) list.
/// gobj.h:8 `HSD_GOBJ_GXLINK_NONE`.
pub const GXLINK_NONE: u8 = 0xFF;
/// `gobj->obj_kind` when no HSD object is attached. gobj.h:9 `HSD_GOBJ_OBJ_NONE`.
pub const OBJ_NONE: u8 = 0xFF;
/// `gobj->user_data_kind` when no user data is attached.
/// gobjuserdata.h:8 `HSD_GOBJ_USER_DATA_NONE` (`(u8) -1`).
pub const USER_DATA_NONE: u8 = 0xFF;

/// `HSD_GObj.classifier` values. gobj.h:11-26.
pub mod class {
    pub const STAGE: u16 = 0x3;
    pub const FIGHTER: u16 = 0x4;
    pub const ITEM: u16 = 0x6;
    /// Chain-type items linking multiple parts (gobj.h:15-16).
    pub const ITEMLINK: u16 = 0x7;
    pub const EFFECT: u16 = 0x8;
    pub const SISLIB_UNK: u16 = 0x9;
    pub const FOG: u16 = 0xA;
    pub const LIGHT: u16 = 0xB;
    pub const GROUND: u16 = 0xD;
    pub const UI: u16 = 0xE;
    pub const TEXT: u16 = 0x11;
    pub const CAMERA: u16 = 0x13;
    pub const SOUND: u16 = 0x17;
}

/// `p_link` list indices used by Melee. `HSD_GObj_Entities` is really
/// `HSD_GObj*[p_link_max + 1]` (gobj.h:111-113); the named `HSD_GObjList`
/// fields are these indices times four (`fighters` at +0x20 is index 8,
/// `items` at +0x24 is index 9).
pub mod plink {
    /// Lights and ground objects: `GObj_Create(HSD_GOBJ_CLASS_LIGHT, 3, 0)`
    /// (if/ifprize.c:187), `GObj_Create(0xD, 3, 0)` (gr/ground.c:2678).
    pub const LIGHT: u8 = 3;
    /// Stage object: `GObj_Create(HSD_GOBJ_CLASS_STAGE, 5, 0)` (gr/ground.c:835).
    pub const STAGE: u8 = 5;
    /// Fighters: `GObj_Create(HSD_GOBJ_CLASS_FIGHTER, 8, 0)` (ft/fighter.c:852).
    /// This is the list `harness/dolphin/walk.py` walks (`fighters` at +0x20).
    pub const FIGHTER: u8 = 8;
    /// Items: `GObj_Create(HSD_GOBJ_CLASS_ITEM, 9, 0)` (it/item.c:957).
    pub const ITEM: u8 = 9;
    /// Item chain links: `GObj_Create(HSD_GOBJ_CLASS_ITEMLINK, 0xAU, 0U)`
    /// (it/kinds/itlinkhookshot.c:220).
    pub const ITEMLINK: u8 = 10;
    /// Effects, gfx ids outside 25..27 (ef/eflib.c:474-475, 481).
    pub const EFFECT: u8 = 11;
    /// Effects, gfx ids 25 and 26 (ef/eflib.c:476-477, 481).
    pub const EFFECT_ALT: u8 = 12;
    /// HUD/UI: `GObj_Create(HSD_GOBJ_CLASS_UI, 15, 0)` (if/ifprize.c:192).
    pub const UI: u8 = 15;
    /// In-game camera helper object: `GObj_Create(0x10, 0x12, 0)` (cm/camera.c:4100).
    pub const CAMERA_HELPER: u8 = 18;
    /// Camera: `GObj_Create(HSD_GOBJ_CLASS_CAMERA, 20, 0)` (gm/*, if/*) and
    /// `GObj_Create(0x13, 0x14, 0)` (vi/vi0402.c:84 and the other `vi` files).
    pub const CAMERA: u8 = 20;
}

/// `gx_link` render list indices used by Melee. Not rendered here; kept so
/// the buckets match.
pub mod gxlink {
    /// `GObj_SetupGXLink(gobj, &ftDrawCommon_80080E18, 5U, 0U)` (ft/fighter.c:853).
    pub const FIGHTER: u8 = 5;
    /// `GObj_SetupGXLink(gobj, ..., 6, ...)` (it/item.c:963, 968).
    pub const ITEM: u8 = 6;
}

/// `s_link` proc priorities registered by Melee. The per-frame loop runs
/// priority 0 first, then 1, ... up to `gproc_pri_max` (gobj.c:101).
pub mod proc_prio {
    /// Melee's `gproc_pri_max`: `gm_80479D48.initdata.gproc_pri_max = 0x18`
    /// (gm/gm_1A45.c:224). HSD's own default is 2 (gobjinit.c:6-10).
    pub const MELEE_MAX: u8 = 0x18;

    /// The fighter procs, in registration order (ft/fighter.c:897-911).
    pub mod fighter {
        /// `Fighter_8006A1BC`
        pub const P0: u8 = 0;
        /// `Fighter_8006A360`
        pub const P1: u8 = 1;
        /// `Fighter_8006ABA0`
        pub const P2: u8 = 2;
        /// `Fighter_Spaghetti_8006AD10`
        pub const P3: u8 = 3;
        /// `Fighter_procUpdate` (the main per-frame update).
        pub const UPDATE: u8 = 4;
        /// `Fighter_procMap`
        pub const MAP: u8 = 6;
        /// `Fighter_8006C5F4`
        pub const P7: u8 = 7;
        /// `Fighter_CallAcessoryCallbacks_8006C624`
        pub const ACCESSORY: u8 = 8;
        /// `Fighter_8006C80C`
        pub const P9: u8 = 9;
        /// `Fighter_UnkProcessGrab_8006CA5C`
        pub const GRAB: u8 = 0xC;
        /// `Fighter_8006CB94`
        pub const P13: u8 = 0xD;
        /// `Fighter_ProcessHit_8006D1EC`
        pub const HIT: u8 = 0xE;
        /// `Fighter_8006D9AC`
        pub const P16: u8 = 0x10;
        /// Fighter camera procedure (retail 0x8006D9EC)
        pub const CAMERA_CB: u8 = 0x12;
        /// `Fighter_8006DA4C`
        pub const P22: u8 = 0x16;
    }

    /// The item procs, in registration order (it/item.c:991-998 and following).
    pub mod item {
        /// `Item_802693E4`
        pub const P0: u8 = 0;
        /// `Item_80269528`
        pub const P1: u8 = 1;
        /// `Item_802697D4`
        pub const P4: u8 = 4;
        /// `Item_80269978`
        pub const P5: u8 = 5;
        /// `Item_80269A9C`
        pub const P9: u8 = 9;
        /// `Item_80269B60`
        pub const P11: u8 = 11;
        /// `Item_80269BE4`
        pub const P12: u8 = 12;
        /// `Item_80269C5C`
        pub const P13: u8 = 13;
    }

    /// Stage: `HSD_GObj_SetupProc(gobj, &Ground_801C1CD0, 1)` and
    /// `HSD_GObj_SetupProc(gobj, &Ground_801C1D38, 4)` (gr/ground.c:933-934);
    /// per-stage callbacks also register at 4.
    pub const GROUND_A: u8 = 1;
    pub const GROUND_B: u8 = 4;
    /// Effects: `HSD_GObj_SetupProc(effect->gobj, efLib_Update, 15)` (ef/eflib.c:534).
    pub const EFFECT: u8 = 15;
    /// Camera: `HSD_GObj_SetupProc(gobj, fn_8002F360, 0x12)` (cm/camera.c:4111).
    pub const CAMERA: u8 = 0x12;
}
