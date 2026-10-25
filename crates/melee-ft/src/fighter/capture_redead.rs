//! Held by a ReDead: ftCo_800C7434.c and ftCo_800C7590.c (800C7434..
//! 800C78B0), the fighter's side of itleadead.c's hold.
//!
//! A ReDead-element hit on a fighter not already held (and not just let go,
//! +2108) takes the ordinary damage entry and then stands in
//! CaptureLeadead: the ReDead clings to it (it_802EAAEC) while a mash timer
//! runs down. When the timer runs out the ReDead lets go (it_802EAE80) and
//! the fighter steps back in CaptureCut; a hit or a death shakes it off
//! (it_802EADD8, the take_dmg_cb and death2_cb). While held a hit deals its
//! damage without a reaction (x2220_b3).
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CollisionPhase, InputPhase},
    Fighter, FighterCore, MotionData, MotionEntryFlags,
};
use crate::{anim::WaitChoice, collision::ground};
use gekko_math::fma::fmadds;
use melee_types::{CommonMotionState as S, FtPart, GroundOrAir, HitElement};

/// PlCo +71C..+73C.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +71C..+730: the mash timer's base, handicap, rank and percent terms.
    pub base_timer: f32,
    pub handicap_scale: f32,
    pub handicap_origin: f32,
    pub rank_scale: f32,
    pub rank_origin: f32,
    pub percent_scale: f32,
    /// +734: what each frame takes off the timer.
    pub decrement: f32,
    /// +738: what each mash input takes off.
    pub mash_decrement: f32,
    /// +73C: frames after a release before a ReDead may catch it again.
    pub recapture_frames: i32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            base_timer: r.f32(base + 0x71C)?,
            handicap_scale: r.f32(base + 0x720)?,
            handicap_origin: r.f32(base + 0x724)?,
            rank_scale: r.f32(base + 0x728)?,
            rank_origin: r.f32(base + 0x72C)?,
            percent_scale: r.f32(base + 0x730)?,
            decrement: r.f32(base + 0x734)?,
            mash_decrement: r.f32(base + 0x738)?,
            recapture_frames: r.s32(base + 0x73C)?,
        })
    }
}

/// The ReDead behind a pending hit (dmg.x1868_source), as the scene saw it
/// before the victim's ProcessHit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReDeadSource {
    /// The item.
    pub item: u32,
    /// it_802EAF28: it already holds a fighter.
    pub holding: bool,
}

/// mv.co.captureleadead and the grab timer with its mash latches.
#[derive(Clone, Debug)]
pub struct CaptureReDeadState {
    /// x0: the ReDead (dmg.x1868_source).
    pub item: u32,
    /// grab_timer (+1A4C).
    pub timer: f32,
    /// x1A50 / x1A51, reset by ftCommon_InitGrab.
    stick_directions: [i8; 2],
    /// mv+4 as the damage entry before left it.
    pub retained_word: Option<f32>,
}

/// Ft_MF_SkipMatAnim | Ft_MF_SkipColAnim (0x1080).
const CAPTURE_FLAGS: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_MAT_ANIM.0 | MotionEntryFlags::SKIP_COL_ANIM.0);

impl Fighter {
    /// ftCo_800C74F4 (800C74F4): a ReDead-element hit holds a fighter whose
    /// recapture wait (+2108) is over, who is not held already, by a ReDead
    /// holding no one (it_802EAF28).
    pub(super) fn redead_holds(&self, element: HitElement) -> bool {
        if element != HitElement::ReDead
            || self.core.status.capture_timer != 0
            || self.core.motion_state.id == S::CaptureLeadead
        {
            return false;
        }
        let Some(source) = self.core.combat.redead_source else {
            unimplemented!("ftCo_800C74F4: a ReDead-element hit from no live ReDead");
        };
        !source.holding
    }

    /// ftCo_800C7590 (800C7590): the ordinary damage entry
    /// (ftCo_8008DCE0(gobj, -1, 0)), then on the floor in CaptureLeadead
    /// with every movement cleared, the mash timer started
    /// (ftCommon_InitGrab, whose shake table only moves the camera's view),
    /// no grab may take it, and the ReDead clings to it.
    pub(super) fn enter_capture_redead(
        &mut self,
        hit: melee_coll::damage::ReceivedHit,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        let source = self
            .core
            .combat
            .redead_source
            .expect("ftCo_800C7590: the ReDead behind the hit");
        self.interrupt_actions(assets);
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C: held by a ReDead with a heavy item");
        }
        // ftCo_800DD168: ftCo_8008EC90 only comes here without a grab link.
        assert!(
            self.core.combat.grab.is_none(),
            "ftCo_800DD168: held by a ReDead in a grab pair"
        );
        self.begin_damage_reaction(hit, None, None, None, false, assets, rng)?;
        // ftCommon_8007D7FC.
        self.land();
        self.change_motion_state_with_flags(
            S::CaptureLeadead.into(),
            assets,
            CAPTURE_FLAGS,
            0.0,
            1.0,
        )?;
        let retained_word = self.inherited_scratch_word();
        self.core.clear_movement();
        let p = &assets.capture_redead;
        // retail 800C763C..B0: the rank term (fsubs, fsubs, fmuls), the
        // handicap term (fsubs, fmadds), fadds, fmadds with percent.
        let rank = f32::from(self.core.standing_rank) + 1.0;
        let rank_term = p.rank_scale * (p.rank_origin - rank);
        let base = fmadds(
            p.handicap_scale,
            p.handicap_origin - f32::from(self.core.grab_handicap),
            p.base_timer,
        );
        let timer = fmadds(self.core.physics.percent, p.percent_scale, base + rank_term);
        self.core.state_data = MotionData::CaptureReDead(CaptureReDeadState {
            item: source.item,
            timer,
            stick_directions: [0; 2],
            retained_word,
        });
        let status = &mut self.core.status;
        status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        status.ignore_fighter_nudge = true;
        status.no_hit_reaction = true;
        self.core
            .item_requests
            .push(melee_it::ItemRequest::ReDeadCling {
                item: source.item,
                part: FtPart::TopN as u8,
            });
        // Camera_RequestQuake(QuakeKind_Small) at cur_pos, after
        // ftCo_8008DCE0's own.
        if self.core.quake_request.is_some() {
            unimplemented!("ftCo_800C7590: two quakes in one capture entry");
        }
        self.core.quake_request = Some(melee_cm::QuakeKind::Small);
        Ok(())
    }

    /// fn_800C74CC (800C74CC) / fn_800C7568 (800C7568), the hold's
    /// take_dmg_cb and death2_cb: the ReDead lets go (it_802EADD8). Every
    /// motion change disarms them. No witness hits or KOs a held fighter
    /// without hitting the ReDead first (a search of Marth's attacks on the
    /// held Fox found none), so the shake-off stays refused.
    pub(super) fn shake_off_redead(&mut self) {
        if self.core.motion_state.id == S::CaptureLeadead {
            unimplemented!(
                "fn_800C74CC / fn_800C7568 (ftCo_800C7434.c:44,68): a held fighter hit or KO'd shakes the ReDead off (it_802EADD8)"
            );
        }
    }

    /// ftCo_800C7800 (800C7800): stepping back out of the hold at PlCo
    /// +370 against the facing, in CaptureCut, safe from another ReDead
    /// for PlCo +73C frames.
    pub fn leave_redead(&mut self, assets: &FighterAssets) -> Result<()> {
        let speed = -self.core.physics.facing * assets.grab_escape.escape_speed;
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.core.physics.ground_velocity = speed;
        } else {
            self.core.physics.self_velocity.x = speed;
        }
        self.change_motion_state(S::CaptureCut.into(), assets)?;
        self.core.status.capture_timer = assets.capture_redead.recapture_frames;
        Ok(())
    }
}

impl FighterCore {
    /// What the ReDead reads of this fighter when it lets go: TopN's world
    /// matrix (ftLib_80086630), the floor line (it_8026C100) and the ECB's
    /// middle and position it_802759DC sweeps from (it_8026BC90).
    pub fn redead_view(&mut self) -> melee_it::CaptiveView {
        let joint = self.animation.parts[FtPart::TopN as usize].joint;
        let part = *self.skeleton.get_mtx(joint);
        let ecb = &self.collision.data.ecb;
        melee_it::CaptiveView {
            part,
            floor_line: self.collision.data.floor.index,
            ecb_middle: 0.5 * (ecb.top.y + ecb.bottom.y),
            position: self.physics.position,
        }
    }
}

/// ftCo_CaptureLeadead_Anim (800C76F0): the timer runs down (PlCo +734)
/// and mashing takes +738 per input; at zero the ReDead lets go
/// (it_802EAE80) and the fighter steps back (ftCo_800C7800).
pub fn capture_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let params = p.assets.capture_redead;
    let threshold = p.assets.grab_escape.stick_threshold;
    let input = f.core.input.clone();
    let MotionData::CaptureReDead(state) = &mut f.core.state_data else {
        panic!("ReDead hold scratch missing")
    };
    state.timer -= params.decrement;
    super::capture_yoshi::grab_mash(
        &mut state.timer,
        &mut state.stick_directions,
        &input,
        threshold,
        params.mash_decrement,
    );
    if state.timer > 0.0 {
        return Ok(None);
    }
    let item = state.item;
    let view = f.core.redead_view();
    f.core
        .item_requests
        .push(melee_it::ItemRequest::ReDeadRelease { item, view });
    f.leave_redead(p.assets)?;
    Ok(None)
}

/// ftCo_CaptureLeadead_IASA: empty.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCo_CaptureLeadead_Coll (800C77C0): ft_8008403C; off the floor,
/// DamageFall (fn_800C77E0 -> ftCo_80090780).
pub fn capture_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    let supported = ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported;
    if !supported {
        f.enter_damage_fall(p.assets.expect("ReDead hold collision assets"))?;
    }
    Ok(())
}
