//! The parasol states and timer shared by the Parasol item and Peach's up
//! special: ftCo_ItemParasolOpen.c, ftCo_ItemParasolFallSpecial.c, the
//! parasol step of Fighter_ChangeMotionState (fighter.c:986-993) and of
//! Fighter_8006A360 (fighter.c:1577-1591), and ftGetParasolStatus.
//!
//! Only a character's own special parasol (Peach's, which she hangs on
//! fp->item_gobj) is ported; the Parasol item's branches are explicit
//! `unimplemented!` boundaries.
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    ActionId, Fighter, MotionData,
};
use crate::anim::WaitChoice;
use melee_types::CommonMotionState;

/// PlCo's parasol parameters (ftCommonData +58C..+59C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParasolParameters {
    /// +58C: the drift and its target, scaled.
    pub drift_multiplier: f32,
    /// +590: gravity and terminal velocity, scaled.
    pub gravity_multiplier: f32,
    /// +594: stick up at least this far opens it from special fall.
    pub open_threshold: f32,
    /// +598: stick down at most this far closes it.
    pub close_threshold: f32,
    /// +59C: frames the parasol may stay open in one airtime.
    pub open_frames: f32,
}
impl ParasolParameters {
    pub fn read(common: &hsd_archive::Archive, data: u32) -> hsd_archive::desc::Result<Self> {
        let r = common.reader();
        Ok(Self {
            drift_multiplier: r.f32(data + 0x58C)?,
            gravity_multiplier: r.f32(data + 0x590)?,
            open_threshold: r.f32(data + 0x594)?,
            close_threshold: r.f32(data + 0x598)?,
            open_frames: r.f32(data + 0x59C)?,
        })
    }
}

/// ftGetParasolStatus (8007E994) for a held special parasol: its motion 1
/// is 4 (opening), motions 0 and 2 are 6 (open).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParasolStatus {
    Opening,
    Open,
}

/// ftCo_ItemParasolGetFallMotionId's Peach arm: the character's own rows
/// while its special parasol hangs on fp->item_gobj.
#[derive(Clone, Copy, Debug)]
pub struct SpecialParasol {
    pub status: ParasolStatus,
    pub open: ActionId,
    pub fall: ActionId,
}

/// Fighter x2221_b4..b7 and x2104.
#[derive(Clone, Debug)]
pub struct ParasolTimer {
    /// x2221_b4: the open timer is counting.
    pub running: bool,
    /// x2221_b5: the timer ran out; no reopening until the ground.
    pub spent: bool,
    /// x2221_b6: the next opening restarts the timer.
    pub fresh: bool,
    /// x2221_b7: an opening needs no stick input (after a jump or on the
    /// ground).
    pub without_stick: bool,
    /// x2104: frames left open.
    pub frames: i32,
}
impl Default for ParasolTimer {
    /// fighter.c:273-276.
    fn default() -> Self {
        Self {
            running: false,
            spent: false,
            fresh: true,
            without_stick: false,
            frames: 0,
        }
    }
}
impl ParasolTimer {
    /// Fighter_ChangeMotionState on the ground, fighter.c:1124-1126.
    pub fn restore_on_ground(&mut self) {
        self.spent = false;
        self.without_stick = true;
        self.fresh = true;
    }
}

/// mv.co.parasol_open: the motion the fighter opened the parasol from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParasolState {
    pub previous: ActionId,
    /// mv+4, which the parasol states never write (`None` where the port
    /// does not model the opening state's word).
    pub retained_word: Option<f32>,
}

impl Fighter {
    /// ftGetParasolStatus reads fp->item_gobj: a parasol knocked out of
    /// the hand no longer counts, although the character still owns it.
    fn special_parasol(&self) -> Option<SpecialParasol> {
        self.core.article_in_hand?;
        (self.character.table().special_parasol)(&self.character)
    }

    /// Fighter_ChangeMotionState, fighter.c:986-993, unless the entry passes
    /// Ft_MF_SkipParasol: the timer stops and an opening parasol snaps open.
    pub(super) fn parasol_motion_change(&mut self) {
        self.core.parasol.running = false;
        if let Some(parasol) = self.special_parasol() {
            if parasol.status != ParasolStatus::Open {
                // ftCommon_8007E83C(gobj, 6, 0.0f).
                (self.character.table().set_parasol_animation)(self, 6, 0.0);
            }
        }
    }

    /// ftAction_80072894 requests queued by this frame's commands.
    pub(super) fn apply_parasol_commands(&mut self) {
        for (index, frames) in std::mem::take(&mut self.core.commands.parasol_animations) {
            if self.special_parasol().is_none() {
                // ftcommon.c:1069: HSD_ASSERT(ftGetParasolStatus != -1).
                unimplemented!("ftCommon_8007E83C: parasol command without a special parasol");
            }
            (self.character.table().set_parasol_animation)(self, index, frames);
        }
    }

    /// ftCo_800CEFE0 (800CEFE0, blend 10) and ft_800CEF08 (800CEF08, blend
    /// 0): open the parasol from `previous`, and on the first opening of an
    /// airtime start its timer.
    pub fn enter_parasol_open(
        &mut self,
        assets: &FighterAssets,
        previous: ActionId,
        blend_frames: f32,
    ) -> Result<()> {
        let Some(parasol) = self.special_parasol() else {
            unimplemented!("ftCo_800CEFE0: ftCo_MS_ItemParasolOpen for the Parasol item");
        };
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state_blended(parasol.open, assets, blend_frames)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Parasol(ParasolState {
            previous,
            retained_word,
        });
        let timer = &mut self.core.parasol;
        timer.without_stick = false;
        timer.running = true;
        if timer.fresh {
            // retail 800CF06C: fctiwz of PlCo +59C.
            timer.frames = gekko_math::msl::fctiwz(assets.parasol.open_frames);
            timer.fresh = false;
        }
        Ok(())
    }

    fn parasol_previous(&self) -> ActionId {
        match self.core.state_data {
            MotionData::Parasol(state) => state.previous,
            _ => panic!("parasol scratch missing"),
        }
    }

    /// ftCo_800CF3C8 (800CF3C8): Ft_MF_SkipHit | Ft_MF_SkipParasol.
    fn enter_parasol_fall_special(&mut self, assets: &FighterAssets) -> Result<()> {
        let Some(parasol) = self.special_parasol() else {
            unimplemented!("ftCo_800CF3C8: ftCo_MS_ItemParasolFallSpecial for the Parasol item");
        };
        let state = self.core.state_data.clone();
        self.change_parasol_fall_motion(parasol.fall, assets)?;
        // mv.co.parasol_open carries over into the special-fall row.
        self.core.state_data = state;
        Ok(())
    }

    /// ftCo_800CEE70 (800CEE70), FallSpecial's first IASA check: with the
    /// parasol held open and falling, reopen it (after a jump or on the
    /// first airtime without the stick, otherwise with the stick up).
    pub(super) fn try_reopen_parasol(&mut self, assets: &FighterAssets) -> Result<bool> {
        let timer = &self.core.parasol;
        if timer.spent {
            return Ok(false);
        }
        // retail 800CEEB0/C4: fcmpo with cror, so both bounds are inclusive.
        if !(timer.without_stick
            || self.core.input.current.stick.y >= assets.parasol.open_threshold)
            || self.core.physics.self_velocity.y > 0.0
            || self.special_parasol().map(|p| p.status) != Some(ParasolStatus::Open)
        {
            return Ok(false);
        }
        let previous = self.core.motion_state.action;
        self.enter_parasol_open(assets, previous, 0.0)?;
        Ok(true)
    }

    /// Fighter_8006A360, fighter.c:1577-1591: the open timer runs out.
    pub fn tick_parasol_timer(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.status.disabled || !self.core.parasol.running {
            return Ok(());
        }
        if self.core.parasol.frames == 0 {
            return Ok(());
        }
        self.core.parasol.frames -= 1;
        if self.core.parasol.frames != 0 {
            return Ok(());
        }
        self.core.parasol.running = false;
        if self.special_parasol().is_none() {
            unimplemented!("fighter.c:1588: ftCo_80095744 drops the Parasol item");
        }
        self.core.parasol.spent = true;
        self.enter_ordinary_special_fall(assets)
    }
}

/// ftCo_ItemParasolOpen_Anim (800CF0B8).
pub fn open_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.parasol_previous() == CommonMotionState::FallSpecial.into() {
            f.enter_parasol_fall_special(p.assets)?;
        } else {
            unimplemented!("ftCo_800CF280: ftCo_MS_ItemParasolFall after an item opening");
        }
    }
    Ok(None)
}

/// ftCo_ItemParasolOpen_IASA (800CF10C): the item opening's specials,
/// throws and air dodge do not apply after special fall.
pub fn open_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.parasol_previous() != CommonMotionState::FallSpecial.into() {
        unimplemented!("ftCo_ItemParasolOpen_IASA: item-parasol special, throw and dodge inputs");
    }
    f.try_aerial_jump(p.assets).expect("parasol aerial jump");
}

/// ftCo_ItemParasolOpen_Phys (800CF1A0), also ftCo_ItemParasol_Phys.
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let parasol = p.assets.parasol;
    let air = &f.core.attributes.air;
    // retail 800CF1D0/D4: two fmuls.
    let gravity = air.gravity * parasol.gravity_multiplier;
    let terminal = air.terminal_velocity * parasol.gravity_multiplier;
    f.core.physics.self_velocity.y =
        crate::physics::airborne::gravity(f.core.physics.self_velocity.y, gravity, terminal);
    let x = f.core.input.current.stick.x;
    // retail 800CF200: fcmpo with cror, inclusive.
    let (drift, target) = if gekko_math::msl::fabsf(x) >= p.assets.jumping.multi_jump_drift_threshold
    {
        // retail 800CF214..224: fmuls stick * attribute, then * PlCo +58C.
        (
            parasol.drift_multiplier * (x * air.air_drift_stick_mul),
            parasol.drift_multiplier * (x * air.air_drift_max),
        )
    } else {
        (0.0, 0.0)
    };
    f.core.physics.animation_velocity.x = crate::physics::airborne::drift_acceleration(
        f.core.physics.self_velocity.x,
        drift,
        target,
        air,
    );
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCo_ItemParasolOpen_Coll (800CF260) -> ft_8008370C(Landing_Enter_Basic).
pub fn open_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("parasol collision assets");
    if parasol_air_collision(f, assets, p.map)? {
        f.enter_landing(assets)?;
    }
    Ok(())
}

/// ftCo_ItemParasolFallSpecial_Anim (800CF40C): nothing.
pub fn fall_special_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftCo_ItemParasolFallSpecial_IASA (800CF410): stick down while the
/// parasol is opening or closing folds it into special fall; otherwise an
/// aerial jump.
pub fn fall_special_input(f: &mut Fighter, p: InputPhase<'_>) {
    // retail 800CF43C: fcmpo with cror, inclusive.
    if f.core.input.current.stick.y <= p.assets.parasol.close_threshold
        && f.special_parasol().map(|parasol| parasol.status) == Some(ParasolStatus::Opening)
    {
        f.enter_ordinary_special_fall(p.assets)
            .expect("parasol close");
    } else {
        f.try_aerial_jump(p.assets).expect("parasol aerial jump");
    }
}

/// ftCo_ItemParasolFallSpecial_Coll (800CF4A8) -> ft_8008370C
/// (ftCo_LandingFallSpecial_Enter_Basic).
pub fn fall_special_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("parasol collision assets");
    if parasol_air_collision(f, assets, p.map)? {
        f.enter_basic_special_landing(assets)?;
    }
    Ok(())
}

/// ft_8008370C (8008370C) without its landing callback: collide, then the
/// wall jump and ledge checks. Returns whether the fighter landed.
fn parasol_air_collision(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<bool> {
    crate::collision::air::begin_map(
        &f.core.physics,
        &mut f.core.collision,
        &mut f.core.skeleton,
        f.core.animation.root,
    );
    if crate::collision::air::collide_pass(
        &mut f.core.physics,
        &mut f.core.collision,
        map,
        &mut f.core.skeleton,
        f.core.animation.root,
        f.core.status.ledge_cooldown == 0,
    ) {
        return Ok(true);
    }
    if !f.try_wall_jump(assets, map)? {
        f.try_grab_ledge(assets, map)?;
    }
    Ok(false)
}
