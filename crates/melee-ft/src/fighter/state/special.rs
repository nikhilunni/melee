//! Special entry boundary shared by the existing attack-input paths.
use crate::fighter::{assets::FighterAssets, Fighter, SpecialSlot};
use crate::input::{WaitContext, WaitPredicate, WaitTransition};

impl Fighter {
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
                if slot == SpecialSlot::Side && !airborne {
                    // ftCo_SpecialS doEnter, 80096614/1C/24: fsubs, fmuls, fmadds.
                    let retention = self
                        .core
                        .attributes
                        .specials
                        .specials_ground_speed_retention;
                    let speed = self.core.physics.ground_velocity;
                    let reduction = -(speed * (1.0 - retention));
                    let terrain =
                        crate::physics::grounded::floor_friction(&self.core.collision.data);
                    self.core.physics.ground_velocity =
                        gekko_math::fma::fmadds(reduction, terrain, speed);
                }
                (self.character.table().enter_special)(self, slot, airborne, assets);
                return;
            }
        }
        unimplemented!("ftCo special input: entry without a supported special buffer");
    }
}
