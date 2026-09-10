//! Special entry boundary shared by the existing attack-input paths.
use crate::fighter::{assets::FighterAssets, CharacterCallbacks, Fighter, SpecialSlot};
use crate::input::{WaitContext, WaitPredicate, WaitTransition};

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_SpecialS_CheckInput / ftCo_Attack100_CheckInput / ftCo_800D6824 /
    /// ftCo_800D68C0 consult ftData_SpecialS/Hi/N/Lw[kind], respectively.
    /// The pure input predicates already recognize these buffers; their entry
    /// bodies are still stubs and no existing gated scene reaches this hook.
    pub(crate) fn enter_buffered_special(&mut self, assets: &FighterAssets, airborne: bool) {
        let context = WaitContext {
            facing: self.core.physics.facing,
            specials_available: self.core.capabilities.specials,
            shield_health: self.core.status.shield_health,
            ..WaitContext::default()
        };
        for (predicate, slot) in [
            (WaitPredicate::SpecialSide, SpecialSlot::Side),
            (WaitPredicate::SpecialUp, SpecialSlot::Up),
            (WaitPredicate::SpecialNeutral, SpecialSlot::Neutral),
            (WaitPredicate::SpecialDown, SpecialSlot::Down),
        ] {
            if crate::input::iasa::evaluate(predicate, &self.core.input, &assets.input, &context)
                == WaitTransition::Special
            {
                C::enter_special(self, slot, airborne);
                return;
            }
        }
        unimplemented!("ftCo special input: entry without a supported special buffer");
    }
}
