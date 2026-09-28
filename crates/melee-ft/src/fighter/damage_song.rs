//! Sleep from a sleep-element hit (Jigglypuff's Sing): DamageSong,
//! DamageSongWait and DamageSongRv (ftCo_DamageSong.c, 800C318C..800C3538).
//! The victim falls asleep where it stands and mashes its way awake.
use super::{
    assets::{FighterAssets, Result},
    shield_break::DizzyState,
    state::AnimationPhase,
    Fighter, MotionData, MotionEntryFlags,
};
use crate::anim::WaitChoice;
use melee_types::CommonMotionState as S;

impl Fighter {
    /// ftCo_800C318C (800C318C), from ftCo_8008E908 for a Nap (`sleep`
    /// false) or Sleep element hit: stop the action sounds, fall asleep and
    /// start the mash timer (ftCommon_InitGrab).
    pub(super) fn enter_damage_song(&mut self, sleep: bool, assets: &FighterAssets) -> Result<()> {
        self.interrupt_actions();
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C: sleeping with a heavy item");
        }
        // ftCo_800DD168: a grab pair would be released.
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800DD168: sleeping in a grab pair");
        }
        // ftCo_DamageSong.c writes no mv field: mv+4 carries through.
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(S::DamageSong.into(), assets)?;
        let p = &assets.grab_escape;
        // 800C31F8..3278: the rank term (fsubs, fsubs, fmuls), the handicap
        // term (fsubs, fmadds), fadds, fmadds with percent; Sleep scales it.
        let rank = f32::from(self.core.standing_rank) + 1.0;
        let rank_term = p.song_rank_scale * (p.song_rank_origin - rank);
        let base = gekko_math::fma::fmadds(
            p.song_handicap_scale,
            p.song_handicap_origin - f32::from(self.core.grab_handicap),
            p.song_base_timer,
        );
        let mut timer = gekko_math::fma::fmadds(
            self.core.physics.percent,
            p.song_percent_scale,
            base + rank_term,
        );
        if sleep {
            timer *= p.sleep_timer_multiplier;
        }
        self.core.state_data = MotionData::Dizzy(DizzyState {
            remaining: timer,
            stick_directions: [0; 2],
            retained_word,
        });
        Ok(())
    }

    /// inlineB0: the timer runs down and mashing speeds it; once spent,
    /// DamageSongRv (ftCo_800C3480). True when it woke.
    fn sleep_mash(&mut self, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) -> Result<bool> {
        let p = &assets.grab_escape;
        let input = self.core.input.clone();
        let MotionData::Dizzy(sleep) = &mut self.core.state_data else {
            panic!("sleep scratch missing")
        };
        sleep.remaining -= p.song_decrement;
        super::capture_yoshi::grab_mash(
            &mut sleep.remaining,
            &mut sleep.stick_directions,
            &input,
            p.stick_threshold,
            p.song_mash_decrement,
        );
        if sleep.remaining > 0.0 {
            return Ok(false);
        }
        self.seal_issued_graphics(assets, rng);
        self.change_motion_state(S::DamageSongRv.into(), assets)?;
        Ok(true)
    }

    /// Fighter_ChangeMotionState's efAsync_QueueFlush (fighter.c:951)
    /// spawns what the outgoing script queued this frame (the sleep
    /// bubbles), ahead of the new script's frame-0 effects. Resolve those
    /// graphics commands (their offset draws are this proc's next RNG work)
    /// so the motion change seals them with the outgoing pose.
    pub fn seal_issued_graphics(&mut self, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        if !self.core.commands.graphics.is_empty() {
            self.core.resolve_graphics_commands(assets, rng);
        }
    }
}

/// ftCo_DamageSong_Anim (800C32AC): asleep; the falling-asleep animation
/// hands over to DamageSongWait (ftCo_800C3390, SkipMatAnim | SkipColAnim).
pub(super) fn fall_asleep_animation(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.sleep_mash(p.assets, p.rng)? && !f.animation.frames_remaining(&f.skeleton) {
        f.seal_issued_graphics(p.assets, p.rng);
        let scratch = std::mem::take(&mut f.core.state_data);
        f.change_motion_state_with_flags(
            S::DamageSongWait.into(),
            p.assets,
            MotionEntryFlags(MotionEntryFlags::SKIP_MAT_ANIM.0 | MotionEntryFlags::SKIP_COL_ANIM.0),
            0.0,
            1.0,
        )?;
        f.core.state_data = scratch;
    }
    Ok(None)
}

/// ftCo_DamageSongWait_Anim (800C33C8).
pub(super) fn asleep_animation(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.sleep_mash(p.assets, p.rng)?;
    Ok(None)
}

/// ftCo_DamageSongRv_Anim (800C34B8): awake, back to Wait (ft_8008A2BC).
pub(super) fn wake_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.seal_issued_graphics(p.assets, p.rng);
        f.change_motion_state(S::Wait.into(), p.assets)?;
    }
    Ok(None)
}
