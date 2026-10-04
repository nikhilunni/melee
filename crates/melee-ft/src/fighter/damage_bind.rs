//! The stun of a Disable-element hit (Mewtwo's Disable): DamageBind
//! (ftCo_DamageBind.c, 800C44CC..800C4744). A grounded victim stands dazed
//! where it is and mashes its way out; an airborne one drops in DamageFall.
use super::{
    assets::{FighterAssets, Result},
    shield_break::DizzyState,
    state::AnimationPhase,
    Fighter, MotionData,
};
use crate::anim::WaitChoice;
use melee_coll::damage::ReceivedHit;
use melee_types::{CommonMotionState as S, GroundOrAir, HitElement};

/// Fighter.x2070's x2071_b0_3 values whose motions take a Disable hit as an
/// ordinary one (ftCo_800C44CC: 5, the dazed states themselves, 9, 11 and
/// 12).
// TODO(meaning): what 9, 11 and 12 group beyond their rows' flag words.
const UNBOUND_MOTION_GROUPS: [u32; 4] = [5, 9, 11, 12];

impl Fighter {
    /// ftCo_800C44CC (800C44CC): whether this hit binds instead of
    /// launching. x2228_b2 (the Sandbag's flag) and the motion groups above
    /// take it as an ordinary hit.
    pub(super) fn binds(&self, element: HitElement) -> bool {
        if element != HitElement::Disable || self.core.status.ledge_grab_disabled {
            return false;
        }
        let flags = self.motion_flags().unwrap_or_else(|| {
            unimplemented!(
                "ftCo_800C44CC: x2071_b0_3 of motion {} is not carried",
                self.core.motion_state.action.0
            )
        });
        !UNBOUND_MOTION_GROUPS.contains(&((flags >> 20) & 0xF))
    }

    /// ftCo_800C4550 (800C4550), after Fighter_UnkTakeDamage_8006CC30: the
    /// action stops (ftCommon_8007DB58), then DamageFall in the air
    /// (ftCo_80090780) or DamageBind with the mash timer
    /// (ftCommon_InitGrab) on the ground.
    pub(super) fn enter_damage_bind(
        &mut self,
        hit: &ReceivedHit,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.core.physics.percent += hit.percent_damage;
        self.interrupt_actions(assets);
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C: bound with a heavy item");
        }
        // ftCo_800DD168: a grab pair would be released.
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800DD168: bound in a grab pair");
        }
        if self.core.physics.ground_or_air == GroundOrAir::Air {
            return self.enter_damage_fall(assets);
        }
        // ftCo_DamageBind.c writes no mv field: mv+4 carries through.
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(S::DamageBind.into(), assets)?;
        let p = &assets.grab_escape;
        // 800C45F4..4644: the rank term (fsubs, fsubs, fmuls), the handicap
        // term (fsubs, fmadds), fadds, fmadds with percent.
        let rank = f32::from(self.core.standing_rank) + 1.0;
        let rank_term = p.bind_rank_scale * (p.bind_rank_origin - rank);
        let base = gekko_math::fma::fmadds(
            p.bind_handicap_scale,
            p.bind_handicap_origin - f32::from(self.core.grab_handicap),
            p.bind_base_timer,
        );
        let timer = gekko_math::fma::fmadds(
            self.core.physics.percent,
            p.bind_percent_scale,
            base + rank_term,
        );
        self.core.state_data = MotionData::Dizzy(DizzyState {
            remaining: timer,
            stick_directions: [0; 2],
            retained_word,
        });
        Ok(())
    }
}

/// ftCo_DamageBind_Anim (800C466C): the timer runs down (fsubs) and mashing
/// speeds it (ftCommon_GrabMash); once spent, Wait (ft_8008A2BC).
pub(super) fn bound_animation(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let parameters = &p.assets.grab_escape;
    let input = f.core.input.clone();
    let MotionData::Dizzy(bind) = &mut f.core.state_data else {
        panic!("bind scratch missing")
    };
    bind.remaining -= parameters.bind_decrement;
    super::capture_yoshi::grab_mash(
        &mut bind.remaining,
        &mut bind.stick_directions,
        &input,
        parameters.stick_threshold,
        parameters.bind_mash_decrement,
    );
    if bind.remaining <= 0.0 {
        f.seal_issued_graphics(p.assets, p.rng);
        f.change_motion_state(S::Wait.into(), p.assets)?;
    }
    Ok(None)
}
